/**
 * Links Meeting — Guest Viewer Application
 *
 * Handles guest joining, LiveKit room connection, and media rendering.
 * Guests can only subscribe to remote tracks; publishing is disabled.
 */

(function () {
  'use strict';

  /* ------------------------------------------------------------------ */
  /*  LiveKit SDK — aliased from UMD global                             */
  /* ------------------------------------------------------------------ */
  const LivekitClient = window.LivekitClient;
  if (!LivekitClient) {
    console.error('[Links] LiveKit client SDK not loaded');
    return;
  }

  const {
    Room,
    RoomEvent,
    Track,
    ConnectionState,
    ParticipantEvent,
  } = LivekitClient;

  /* ------------------------------------------------------------------ */
  /*  DOM references                                                    */
  /* ------------------------------------------------------------------ */
  const $joinView         = document.getElementById('join-view');
  const $meetingView      = document.getElementById('meeting-view');
  const $displayMeetingNo = document.getElementById('display-meeting-no');
  const $passwordGroup    = document.getElementById('password-group');
  const $inputPassword    = document.getElementById('input-password');
  const $btnJoin          = document.getElementById('btn-join');
  const $joinError        = document.getElementById('join-error');
  const $meetingTopic     = document.getElementById('meeting-topic');
  const $meetingNoDisplay = document.getElementById('meeting-no-display');
  const $meetingDuration  = document.getElementById('meeting-duration');
  const $videoGrid        = document.getElementById('video-grid');
  const $emptyState       = document.getElementById('empty-state');
  const $btnToggleMembers = document.getElementById('btn-toggle-members');
  const $memberCount      = document.getElementById('member-count');
  const $sidebar          = document.getElementById('sidebar');
  const $btnCloseSidebar  = document.getElementById('btn-close-sidebar');
  const $memberList       = document.getElementById('member-list');
  const $btnLeave         = document.getElementById('btn-leave');

  /* ------------------------------------------------------------------ */
  /*  State                                                             */
  /* ------------------------------------------------------------------ */
  let meetingNo   = '';
  let room        = null;
  let joinedAt    = null;
  let durationTimer = null;

  /** Map<participantSid, { identity, name, isHost, element }> */
  const participants = new Map();

  /** Map<trackSid, HTMLElement> — video tile elements by track */
  const videoTiles = new Map();

  /* ------------------------------------------------------------------ */
  /*  URL parsing                                                       */
  /* ------------------------------------------------------------------ */
  function init() {
    const params = new URLSearchParams(window.location.search);
    meetingNo = (params.get('meetingNo') || '').trim();

    if (!meetingNo || !/^\d{9}$/.test(meetingNo)) {
      showJoinError('无效的会议号，请检查链接是否正确。');
      $btnJoin.disabled = true;
      $displayMeetingNo.textContent = '—';
      return;
    }

    // Format: 123 456 789
    $displayMeetingNo.textContent = meetingNo.replace(/(\d{3})(\d{3})(\d{3})/, '$1 $2 $3');

    // Bind events
    $btnJoin.addEventListener('click', handleJoin);
    $inputPassword.addEventListener('keydown', (e) => { if (e.key === 'Enter') handleJoin(); });
    $btnLeave.addEventListener('click', handleLeave);
    $btnToggleMembers.addEventListener('click', toggleSidebar);
    $btnCloseSidebar.addEventListener('click', () => $sidebar.classList.remove('open'));
  }

  /* ------------------------------------------------------------------ */
  /*  API helpers                                                       */
  /* ------------------------------------------------------------------ */
  const API_BASE = window.location.origin;

  async function apiGuestJoin(password) {
    const body = {};
    if (password) body.meetingPassword = password;

    const res = await fetch(`${API_BASE}/api/meetings/${meetingNo}/guest-join`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    const data = await res.json();
    if (!res.ok) {
      const errorMsg = data.error || '加入失败';
      const code     = data.code  || '';
      throw { message: errorMsg, code, status: res.status };
    }
    return data;
  }

  /* ------------------------------------------------------------------ */
  /*  Join flow                                                         */
  /* ------------------------------------------------------------------ */
  async function handleJoin() {
    clearJoinError();
    $btnJoin.disabled = true;
    $btnJoin.innerHTML = '<span class="spinner"></span> 正在加入…';

    try {
      const password = $inputPassword.value.trim() || undefined;
      const result   = await apiGuestJoin(password);

      // Connect to LiveKit room
      await connectRoom(result.token, result.url, result.roomName, result.meetingNo);
    } catch (err) {
      handleJoinError(err);
    } finally {
      $btnJoin.disabled = false;
      $btnJoin.innerHTML = '加入会议';
    }
  }

  function handleJoinError(err) {
    const code = err.code || '';
    const msg  = err.message || '未知错误';

    const errorMessages = {
      'PASSWORD_REQUIRED': '该会议需要密码，请输入会议密码。',
      'PASSWORD_INVALID':  '会议密码错误，请重新输入。',
      'GUEST_NOT_ALLOWED': '该会议不允许游客加入。',
      'HOST_NOT_JOINED':   '主持人尚未开启会议，请稍后再试。',
      'MEETING_NOT_STARTED': '会议尚未到预定时间，请稍后再试。',
      'MEETING_ENDED':     '该会议已结束。',
      'MEETING_CANCELLED': '该会议已取消。',
    };

    if (code === 'PASSWORD_REQUIRED' || code === 'PASSWORD_INVALID') {
      $passwordGroup.classList.remove('hidden');
      $inputPassword.focus();
    }

    showJoinError(errorMessages[code] || msg);
  }

  function showJoinError(msg) {
    $joinError.textContent = msg;
  }

  function clearJoinError() {
    $joinError.textContent = '';
  }

  /* ------------------------------------------------------------------ */
  /*  LiveKit Room connection                                           */
  /* ------------------------------------------------------------------ */
  async function connectRoom(token, url, roomName, meetingNumber) {
    room = new Room({
      adaptiveStream: true,
      dynacast: false, // guest doesn't publish
    });

    // Room events
    room.on(RoomEvent.Connected, () => {
      console.log('[Links] Connected to room');
      onRoomConnected(roomName, meetingNumber);
    });

    room.on(RoomEvent.Disconnected, (reason) => {
      console.log('[Links] Disconnected:', reason);
      onRoomDisconnected();
    });

    room.on(RoomEvent.TrackSubscribed, (track, publication, participant) => {
      onTrackSubscribed(track, publication, participant);
    });

    room.on(RoomEvent.TrackUnsubscribed, (track, publication, participant) => {
      onTrackUnsubscribed(track, publication, participant);
    });

    room.on(RoomEvent.ParticipantConnected, (participant) => {
      addParticipant(participant);
      refreshUI();
    });

    room.on(RoomEvent.ParticipantDisconnected, (participant) => {
      removeParticipant(participant);
      refreshUI();
    });

    room.on(RoomEvent.ConnectionStateChanged, (state) => {
      console.log('[Links] Connection state:', state);
    });

    // Connect
    await room.connect(url, token);
  }

  /* ------------------------------------------------------------------ */
  /*  Room lifecycle                                                    */
  /* ------------------------------------------------------------------ */
  function onRoomConnected(roomName, meetingNumber) {
    // Switch views
    $joinView.style.display = 'none';
    $meetingView.classList.add('active');

    // Set meeting info
    $meetingNoDisplay.textContent = '会议号: ' + (meetingNumber || meetingNo);
    joinedAt = Date.now();
    startDurationTimer();

    // Populate existing participants
    room.remoteParticipants.forEach((p) => addParticipant(p));

    // Resolve meeting topic from room metadata if available
    try {
      const meta = room.metadata ? JSON.parse(room.metadata) : {};
      if (meta.topic) {
        $meetingTopic.textContent = meta.topic;
      }
    } catch (_) { /* ignore */ }

    refreshUI();
  }

  function onRoomDisconnected() {
    stopDurationTimer();
    participants.clear();
    videoTiles.forEach((el) => el.remove());
    videoTiles.clear();

    // Return to join view
    $meetingView.classList.remove('active');
    $joinView.style.display = '';
    showJoinError('已断开连接。');
    room = null;
  }

  /* ------------------------------------------------------------------ */
  /*  Duration timer                                                    */
  /* ------------------------------------------------------------------ */
  function startDurationTimer() {
    updateDuration();
    durationTimer = setInterval(updateDuration, 1000);
  }

  function stopDurationTimer() {
    if (durationTimer) {
      clearInterval(durationTimer);
      durationTimer = null;
    }
  }

  function updateDuration() {
    if (!joinedAt) return;
    const elapsed = Math.floor((Date.now() - joinedAt) / 1000);
    const h  = Math.floor(elapsed / 3600);
    const m  = Math.floor((elapsed % 3600) / 60);
    const s  = elapsed % 60;
    const parts = [];
    if (h > 0) parts.push(String(h).padStart(2, '0'));
    parts.push(String(m).padStart(2, '0'));
    parts.push(String(s).padStart(2, '0'));
    $meetingDuration.textContent = parts.join(':');
  }

  /* ------------------------------------------------------------------ */
  /*  Participant management                                            */
  /* ------------------------------------------------------------------ */
  function addParticipant(participant) {
    let isHost = false;
    try {
      const meta = participant.metadata ? JSON.parse(participant.metadata) : {};
      isHost = !!meta.isHost;
    } catch (_) { /* ignore */ }

    participants.set(participant.sid, {
      identity: participant.identity,
      name:     participant.name || participant.identity,
      isHost,
    });
  }

  function removeParticipant(participant) {
    participants.delete(participant.sid);

    // Remove video tiles for this participant
    const tilesToRemove = [];
    videoTiles.forEach((el, trackSid) => {
      if (el.dataset.participantSid === participant.sid) {
        tilesToRemove.push(trackSid);
      }
    });
    tilesToRemove.forEach((sid) => {
      videoTiles.get(sid)?.remove();
      videoTiles.delete(sid);
    });
  }

  /* ------------------------------------------------------------------ */
  /*  Track subscription                                                */
  /* ------------------------------------------------------------------ */
  function onTrackSubscribed(track, publication, participant) {
    if (track.kind === Track.Kind.Video) {
      attachVideoTrack(track, publication, participant);
    } else if (track.kind === Track.Kind.Audio) {
      attachAudioTrack(track, participant);
    }
    refreshUI();
  }

  function onTrackUnsubscribed(track, publication, participant) {
    if (track.kind === Track.Kind.Video) {
      detachVideoTrack(track, publication);
    } else if (track.kind === Track.Kind.Audio) {
      detachAudioTrack(track);
    }
    refreshUI();
  }

  function attachVideoTrack(track, publication, participant) {
    const existingTile = videoTiles.get(publication.trackSid);
    if (existingTile) {
      existingTile.remove();
      videoTiles.delete(publication.trackSid);
    }

    const tile = document.createElement('div');
    tile.className = 'video-tile';
    tile.dataset.participantSid = participant.sid;
    tile.dataset.trackSid = publication.trackSid;

    const videoEl = track.attach();
    videoEl.setAttribute('playsinline', '');
    videoEl.setAttribute('autoplay', '');
    tile.appendChild(videoEl);

    // Label
    const label = document.createElement('div');
    label.className = 'video-tile__label';

    const nameSpan = document.createElement('span');
    nameSpan.className = 'video-tile__name';
    nameSpan.textContent = participant.name || participant.identity;
    label.appendChild(nameSpan);

    // Source label (camera vs screen)
    if (publication.source === Track.Source.ScreenShare) {
      const srcBadge = document.createElement('span');
      srcBadge.className = 'video-tile__host-badge';
      srcBadge.textContent = '屏幕共享';
      label.appendChild(srcBadge);
    }

    // Host badge
    let isHost = false;
    try {
      const meta = participant.metadata ? JSON.parse(participant.metadata) : {};
      isHost = !!meta.isHost;
    } catch (_) {}
    if (isHost) {
      const badge = document.createElement('span');
      badge.className = 'video-tile__host-badge';
      badge.textContent = '主持人';
      label.appendChild(badge);
    }

    tile.appendChild(label);
    $videoGrid.appendChild(tile);
    videoTiles.set(publication.trackSid, tile);
  }

  function detachVideoTrack(track, publication) {
    track.detach().forEach((el) => el.remove());
    const tile = videoTiles.get(publication.trackSid);
    if (tile) {
      tile.remove();
      videoTiles.delete(publication.trackSid);
    }
  }

  function attachAudioTrack(track, participant) {
    const audioEl = track.attach();
    audioEl.id = `audio-${track.sid}`;
    audioEl.setAttribute('autoplay', '');
    // Hide audio elements but keep them in DOM
    audioEl.style.display = 'none';
    document.body.appendChild(audioEl);
  }

  function detachAudioTrack(track) {
    track.detach().forEach((el) => el.remove());
    const existingEl = document.getElementById(`audio-${track.sid}`);
    if (existingEl) existingEl.remove();
  }

  /* ------------------------------------------------------------------ */
  /*  UI refresh                                                        */
  /* ------------------------------------------------------------------ */
  function refreshUI() {
    // Update video tile count for grid layout
    const tileCount = videoTiles.size;
    $videoGrid.dataset.count = String(Math.min(tileCount, 9));

    // Empty state
    if (tileCount === 0) {
      $emptyState.classList.remove('hidden');
    } else {
      $emptyState.classList.add('hidden');
    }

    // Member count (remote participants + 1 for self)
    const totalParticipants = participants.size + 1;
    $memberCount.textContent = String(totalParticipants);

    // Rebuild member list
    renderMemberList();
  }

  function renderMemberList() {
    $memberList.innerHTML = '';

    // Self (guest)
    const selfItem = createMemberItem('我（游客）', false, true);
    $memberList.appendChild(selfItem);

    // Remote participants
    const sorted = [...participants.values()].sort((a, b) => {
      if (a.isHost !== b.isHost) return a.isHost ? -1 : 1;
      return a.name.localeCompare(b.name);
    });

    sorted.forEach((p) => {
      const item = createMemberItem(p.name, p.isHost, false);
      $memberList.appendChild(item);
    });
  }

  function createMemberItem(name, isHost, isSelf) {
    const item = document.createElement('div');
    item.className = 'member-item';

    const avatar = document.createElement('div');
    avatar.className = 'member-item__avatar';
    avatar.textContent = getInitial(name);

    const info = document.createElement('div');
    info.className = 'member-item__info';

    const nameEl = document.createElement('div');
    nameEl.className = 'member-item__name';
    nameEl.textContent = name;

    const roleEl = document.createElement('div');
    if (isHost) {
      roleEl.className = 'member-item__role member-item__role--host';
      roleEl.textContent = '主持人';
    } else if (isSelf) {
      roleEl.className = 'member-item__role';
      roleEl.textContent = '游客（仅观看）';
    } else {
      roleEl.className = 'member-item__role';
      roleEl.textContent = '参与者';
    }

    info.appendChild(nameEl);
    info.appendChild(roleEl);

    const dot = document.createElement('div');
    dot.className = 'status-dot status-dot--connected';

    item.appendChild(avatar);
    item.appendChild(info);
    item.appendChild(dot);

    return item;
  }

  function getInitial(name) {
    if (!name) return '?';
    // Try first character (works for CJK and Latin)
    return name.charAt(0).toUpperCase();
  }

  /* ------------------------------------------------------------------ */
  /*  Sidebar toggle                                                    */
  /* ------------------------------------------------------------------ */
  function toggleSidebar() {
    $sidebar.classList.toggle('open');
  }

  /* ------------------------------------------------------------------ */
  /*  Leave meeting                                                     */
  /* ------------------------------------------------------------------ */
  async function handleLeave() {
    if (room) {
      await room.disconnect();
    }
  }

  /* ------------------------------------------------------------------ */
  /*  Boot                                                              */
  /* ------------------------------------------------------------------ */
  document.addEventListener('DOMContentLoaded', init);
})();

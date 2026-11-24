// ignore_for_file: file_names

/// Chains unused vertical page input to the owning Flutter scrollable.
const composeWebViewScrollBridgeScript = r'''
(function() {
  if (window.__operitComposeScrollBridgeInstalled) return;
  window.__operitComposeScrollBridgeInstalled = true;
  var pending = 0;
  var frame = null;
  var touch = null;
  // Send input without creating RPC promises or triggering a Compose render.
  function send(phase, delta) {
    window.__ComposeDslWebViewHostBridgeChannel__.postMessage(JSON.stringify({type: 'scrollBoundary', payload: {
      phase: phase, delta: delta, viewportHeight: window.innerHeight
    }}));
  }
  // Flush accumulated wheel input at most once per display frame.
  function flush() {
    frame = null;
    var delta = pending;
    pending = 0;
    if (delta) send('wheel', delta);
  }
  // Inspect the hit element and every scrollable ancestor, including shadow DOM.
  function remainder(event, delta) {
    var path = event.composedPath();
    var root = document.scrollingElement;
    for (var i = 0; i < path.length && delta; i++) {
      var element = path[i];
      if (!(element instanceof Element)) continue;
      if (element !== root && !/^(auto|scroll|overlay)$/.test(getComputedStyle(element).overflowY)) continue;
      var maximum = Math.max(0, element.scrollHeight - element.clientHeight);
      var top = Math.max(0, Math.min(maximum, element.scrollTop));
      var capacity = delta > 0 ? maximum - top : top;
      delta = Math.sign(delta) * Math.max(0, Math.abs(delta) - capacity);
    }
    return delta;
  }
  // Retain browser scrolling and forward only the amount outside its bounds.
  window.addEventListener('wheel', function(event) {
    if (!window.__ComposeDslWebViewHostBridgeChannel__) return;
    if (event.defaultPrevented || event.ctrlKey || Math.abs(event.deltaX) > Math.abs(event.deltaY)) return;
    var style = getComputedStyle(event.target);
    var lineHeight = style.lineHeight === 'normal' ? parseFloat(style.fontSize) * 1.2 : parseFloat(style.lineHeight);
    var unit = event.deltaMode === 1 ? lineHeight :
      event.deltaMode === 2 ? window.innerHeight : 1;
    var delta = event.deltaY * unit;
    var unused = remainder(event, delta);
    if (!unused) return;
    if (unused === delta && event.cancelable) event.preventDefault();
    pending += unused;
    if (frame === null) frame = requestAnimationFrame(flush);
  }, {passive: false});
  // Track one-finger scrolling without claiming taps, horizontal drags or pinch zoom.
  window.addEventListener('touchstart', function(event) {
    if (!window.__ComposeDslWebViewHostBridgeChannel__) return;
    if (event.touches.length !== 1) { touch = null; send('cancel', 0); return; }
    var point = event.touches[0];
    touch = {id: point.identifier, x: point.screenX, y: point.screenY};
    send('start', 0);
  }, {passive: true});
  window.addEventListener('touchmove', function(event) {
    if (!touch || event.touches.length !== 1) return;
    var point = event.touches[0];
    if (point.identifier !== touch.id) return;
    var delta = touch.y - point.screenY;
    var horizontal = touch.x - point.screenX;
    touch.x = point.screenX;
    touch.y = point.screenY;
    if (event.defaultPrevented || Math.abs(horizontal) > Math.abs(delta)) return;
    var unused = remainder(event, delta);
    if (!unused) return;
    if (unused === delta && event.cancelable) event.preventDefault();
    send('move', unused);
  }, {passive: false});
  // Release the outer scroll activity when the page gesture finishes.
  function finish(event) {
    if (!touch) return;
    touch = null;
    send(event.type === 'touchcancel' ? 'cancel' : 'end', 0);
  }
  window.addEventListener('touchend', finish, {passive: true});
  window.addEventListener('touchcancel', finish, {passive: true});
})();
''';

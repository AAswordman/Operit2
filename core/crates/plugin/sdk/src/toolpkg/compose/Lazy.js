/// Builds only the requested list window using the page's existing hook and action ownership.
var OperitComposeLazy = function () {
  /// Materializes fixed-extent items without storing nodes or callbacks in a second registry.
  function items(ctx, options, itemBuilder) {
    var key = options.key;
    var count = options.itemCount;
    var extent = options.itemExtent;
    var initial = options.initialItemCount;
    if (typeof key !== 'string' || !key || !Number.isInteger(count) || count < 0 ||
        !Number.isFinite(extent) || extent <= 0 || !Number.isInteger(initial) || initial <= 0 ||
        typeof itemBuilder !== 'function') throw new TypeError('Invalid Compose lazy item options');
    var stored = ctx.useRef('compose:lazy:' + key, { firstIndex: 0, lastIndex: initial });
    var first = Math.min(stored.current.firstIndex, Math.max(0, count - initial));
    var last = Math.min(count, Math.max(first, stored.current.lastIndex,
      stored.current.itemCount !== count ? first + initial : first));
    // Persist the rendered range, not a range from a larger pre-collapse catalog.
    // Growing a short catalog must also refill in this commit, before Flutter asks.
    stored.current = { firstIndex: first, lastIndex: last, itemCount: count };
    // The engine owns invalidation. Viewport-only actions keep this revision;
    // state, host input, source and ordinary callback changes invalidate it.
    var revision = OperitComposeReactive.contentRevision(ctx);
    var inputs = ctx.useRef('compose:lazy-inputs:' + key, null);
    if (!inputs.current || inputs.current.revision !== revision ||
        inputs.current.count !== count || inputs.current.extent !== extent) {
      inputs.current = { revision: revision, count: count, extent: extent };
    }
    var version = inputs.current;
    function materializeItem(index) {
      return OperitComposeReactive.memoItem(ctx,
        'compose:lazy-item:' + key + ':' + index, version,
        function () { return itemBuilder(index); });
    }
    function materialize() {
      var content = [];
      for (var index = first; index < last; index += 1) {
        content.push(materializeItem(index));
      }
      return content;
    }
    // Overlapping items keep their own generation and callbacks. The window's
    // memo region releases items as soon as they leave it, so cache size stays
    // bounded by materialized rows rather than the full catalog.
    var content = ctx.useMemo('compose:lazy-content:' + key, materialize, [first, last, count, extent, revision]);
    return {
      content: content,
      viewport: {
        firstIndex: first,
        itemCount: count,
        itemExtent: extent,
        /// Invalidates the owning composition after a validated viewport range change.
        onRangeChanged: OperitComposeActions.viewport(function (range) {
          if (!range || !Number.isInteger(range.firstIndex) || !Number.isInteger(range.lastIndex) ||
              range.firstIndex < 0 || range.lastIndex < range.firstIndex) {
            throw new TypeError('Invalid Compose lazy item range');
          }
          var liveCount = stored.current.itemCount;
          var next = { firstIndex: Math.min(range.firstIndex, liveCount), lastIndex: Math.min(range.lastIndex, liveCount), itemCount: liveCount };
          if (next.firstIndex === stored.current.firstIndex && next.lastIndex === stored.current.lastIndex) return;
          stored.current = next;
          OperitComposeReactive.invalidateViewport(ctx);
        })
      }
    };
  }
  return { items: items };
}();

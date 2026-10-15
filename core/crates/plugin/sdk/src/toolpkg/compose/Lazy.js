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
    var last = Math.min(count, Math.max(first, stored.current.lastIndex));
    var content = [];
    for (var index = first; index < last; index += 1) content.push(itemBuilder(index));
    return {
      content: content,
      viewport: {
        firstIndex: first,
        itemCount: count,
        itemExtent: extent,
        /// Invalidates the owning composition after a validated viewport range change.
        onRangeChanged: function (range) {
          if (!range || !Number.isInteger(range.firstIndex) || !Number.isInteger(range.lastIndex) ||
              range.firstIndex < 0 || range.lastIndex < range.firstIndex) {
            throw new TypeError('Invalid Compose lazy item range');
          }
          var next = { firstIndex: Math.min(range.firstIndex, count), lastIndex: Math.min(range.lastIndex, count) };
          if (next.firstIndex === stored.current.firstIndex && next.lastIndex === stored.current.lastIndex) return;
          stored.current = next;
          OperitComposeReactive.invalidateRoot(ctx);
        }
      }
    };
  }
  return { items: items };
}();

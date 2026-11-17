"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.automationValue = automationValue;
exports.automationSegment = automationSegment;
/** Linear sample-accurate parameter envelopes, clamped at the first and last breakpoint. */
function automationValue(points, beat) {
    if (beat <= points[0].beat)
        return points[0].value;
    let left = 0, right = points.length - 1;
    if (beat >= points[right].beat)
        return points[right].value;
    while (right - left > 1) {
        const mid = (left + right) >>> 1;
        if (points[mid].beat > beat)
            right = mid;
        else
            left = mid;
    }
    const a = points[left], b = points[right];
    return a.value + (b.value - a.value) * (beat - a.beat) / (b.beat - a.beat);
}
/** Bounded segments can be shared by live loop/seek playback and offline export. */
function automationSegment(points, start, end) {
    return [{ beat: start, value: automationValue(points, start) }, ...points.filter(p => p.beat > start && p.beat < end), { beat: end, value: automationValue(points, end) }];
}

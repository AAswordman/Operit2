import type { Note, Project, Track } from "../../src/shared/model";
/** Derived phrases, not a second copy of notes or a new project format. */
export interface Region { key: string; trackId: string; name: string; start: number; end: number }
export function trackRegions(track: Track, project: Project): Region[] {
  const total = project.bars * project.beatsPerBar, bar = project.beatsPerBar;
  const boundaries = [...new Set([0, total, ...project.sections.flatMap(s => [s.start, s.start + s.length])])].filter(b => b >= 0 && b <= total).sort((a, b) => a - b);
  const notes = [...track.notes].sort((a, b) => a.start - b.start);
  const regions: Region[] = [];
  let index = 0;
  for (let segment = 0; segment < boundaries.length - 1; segment++) {
    const from = boundaries[segment], to = boundaries[segment + 1];
    let start = -1, end = -1;
    const flush = (): void => {
      if (start < 0) return;
      const section = project.sections.find(s => start >= s.start && start < s.start + s.length);
      regions.push({ key: `${track.id}:${start}:${end}`, trackId: track.id, name: section?.name ?? `乐段 ${regions.length + 1}`, start, end });
    };
    while (index < notes.length && notes[index].start < to) {
      const n = notes[index++];
      const noteStart = Math.max(from, Math.floor(n.start / bar) * bar);
      const noteEnd = Math.min(to, Math.ceil((n.start + n.duration) / bar) * bar);
      if (start >= 0 && noteStart > end) { flush(); start = -1; }
      if (start < 0) { start = noteStart; end = noteEnd; }
      else end = Math.max(end, noteEnd);
    }
    flush();
  }
  return regions;
}
/** Note ownership uses start time; sustaining notes from a previous region aren't edited here. */
export function regionNotes(track: Track, region: Region): Note[] {
  return track.notes.filter(n => n.start >= region.start && n.start < region.end);
}
export function regionWindow(region: Region, page: number, visibleBars: number, beatsPerBar: number): { start: number; end: number } {
  const start = Math.max(region.start, Math.min(page * beatsPerBar, Math.max(region.start, (Math.ceil(region.end / beatsPerBar)-1)*beatsPerBar)));
  return { start, end: Math.min(region.end, start + visibleBars * beatsPerBar) };
}

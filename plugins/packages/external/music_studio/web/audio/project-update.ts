import type { Project } from '../../src/shared/model';
/** Notes, metadata, tempo and ordinary parameter edits do not require a fresh DSP instance. */
export function canUpdateInPlace(before: Project, after: Project): boolean {
  return before.id===after.id && before.tracks.length===after.tracks.length && before.tracks.every((t,i)=>{
    const next=after.tracks[i];
    return t.id===next.id && JSON.stringify(t.effects.map(e=>[e.id,e.type,e.enabled]))===JSON.stringify(next.effects.map(e=>[e.id,e.type,e.enabled]));
  });
}

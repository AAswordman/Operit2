import { copy, type Project } from '../../src/shared/model';
/** A temporary solo routing snapshot, never saved or sent through project operations. */
export function auditionProject(project: Project, trackId: string): Project {
  const track=project.tracks.find(t=>t.id===trackId); if(!track) throw Error('试听轨道不存在');
  return { ...project,loop:{ ...project.loop,enabled:false },tracks:[{ ...copy(track),notes:[],automation:[],mute:false,solo:false }] };
}

import { copy, noteName, type Project, type Operation, type Request, type PlaybackStatus } from '../src/shared/model';
import { applyOperations } from '../src/shared/operations';
export interface VersionRef { projectId: string; revision: number }
interface Range { fromBar: number; toBar: number }
interface ChangeInput extends VersionRef { operations: Operation[]; label: string }
interface RelativeNote { pitch: number; offset: number; duration: number; velocity: number; id?: string }
export interface LiveControlHost {
  read(): Promise<Project>;
  write(request: Request, label: string): Promise<Project>;
  status(): PlaybackStatus;
  transport(type: 'play'|'pause'|'stop'|'seek', beat?: number): Promise<void>;
  view(type: 'arrangement'|'instrument'|'effects'|'piano', trackId?: string, beat?: number): void;
  catalog(): Promise<unknown>;
}
const version=(p: Project): VersionRef=>({ projectId:p.id,revision:p.revision });
function expect(project: Project, ref: VersionRef): void {
  if(ref.projectId!==project.id||ref.revision!==project.revision)throw Error(`REVISION_CONFLICT: read current webpage first; projectId=${project.id}, revision=${project.revision}`);
}
function range(project: Project, input: Range): { start: number; end: number } {
  if(!Number.isInteger(input.fromBar)||!Number.isInteger(input.toBar)||input.fromBar<1||input.toBar<input.fromBar||input.toBar>project.bars)throw Error(`小节范围必须是 1–${project.bars} 内的整数，toBar 包含在范围内`);
  return { start:(input.fromBar-1)*project.beatsPerBar,end:input.toBar*project.beatsPerBar };
}
function track(project: Project, id: string) { const t=project.tracks.find(t=>t.id===id); if(!t)throw Error(`Track not found: ${id}`); return t; }
function validateChanges(input: ChangeInput): void {
  if(typeof input.label!=='string'||!input.label.trim()||input.label.length>200)throw Error('每次修改需要 1–200 字的 label，说明本轮指导与修改');
  if(!Array.isArray(input.operations)||!input.operations.length)throw Error('需要非空 operations');
  // This collaboration API never invents musical content through pattern generators.
  if(input.operations.some(op=>op.type==='pattern.generate'))throw Error('协作接口不自动生成音型；请提交明确的音符或局部修改');
}
function summary(before: Project,after: Project) {
  const changed=new Set([...before.tracks.map(t=>t.id),...after.tracks.map(t=>t.id)]);
  const tracks=[...changed].flatMap(id=>{
    const a=before.tracks.find(t=>t.id===id),b=after.tracks.find(t=>t.id===id);
    if(JSON.stringify(a)===JSON.stringify(b))return [];
    const old=new Map(a?.notes.map(n=>[n.id,n])??[]),next=new Map(b?.notes.map(n=>[n.id,n])??[]);
    return [{ trackId:id,name:b?.name??a!.name,added:!a,removed:!b,
      notesAdded:[...next.keys()].filter(id=>!old.has(id)).length,
      notesRemoved:[...old.keys()].filter(id=>!next.has(id)).length,
      notesChanged:[...next.keys()].filter(id=>old.has(id)&&JSON.stringify(old.get(id))!==JSON.stringify(next.get(id))).length,
      soundChanged:JSON.stringify(a && { synth:a.synth,effects:a.effects,gain:a.gain,pan:a.pan,mute:a.mute,solo:a.solo,automation:a.automation })!==JSON.stringify(b && { synth:b.synth,effects:b.effects,gain:b.gain,pan:b.pan,mute:b.mute,solo:b.solo,automation:b.automation }),
    }];
  });
  return { previousRevision:before.revision,...version(after),name:after.name,tracks,
    arrangementChanged:JSON.stringify({ bpm:before.bpm,bars:before.bars,beatsPerBar:before.beatsPerBar,swing:before.swing,loop:before.loop,sections:before.sections,masterGain:before.masterGain })!==JSON.stringify({ bpm:after.bpm,bars:after.bars,beatsPerBar:after.beatsPerBar,swing:after.swing,loop:after.loop,sections:after.sections,masterGain:after.masterGain }) };
}
/** Explicitly opt-in, localhost-only automation bridge for the very page the user is hearing. */
export function createLiveControl(host: LiveControlHost) {
  const api={
    version:1 as const,
    async get() {
      const p=await host.read();
      return { ...version(p),name:p.name,bpm:p.bpm,bars:p.bars,beatsPerBar:p.beatsPerBar,loop:copy(p.loop),sections:copy(p.sections),playback:copy(host.status()),
        tracks:p.tracks.map(t=>({ id:t.id,name:t.name,preset:t.preset,engine:t.synth.engine,noteCount:t.notes.length,gain:t.gain,pan:t.pan,mute:t.mute,solo:t.solo })) };
    },
    async project() { return copy(await host.read()); },
    async newProject(input: { name: string }) {
      const before=await host.read();
      if(typeof input?.name!=="string"||!input.name.trim()||input.name.length>110)throw Error("新工程名称必须是 1–110 个字符");
      const after=await host.write({ action:"create",...version(before),name:input.name.trim() },`协作 · 新建工程：${input.name.trim()}`);
      return { ...version(after),name:after.name,bars:after.bars,tracks:after.tracks.map(t=>({ id:t.id,name:t.name,preset:t.preset })) };
    },
    catalog:()=>host.catalog(),
    async notes(input: { trackId: string } & Partial<Range>) {
      const p=await host.read(),t=track(p,input.trackId);
      const window=range(p,{ fromBar:input.fromBar??1,toBar:input.toBar??p.bars });
      return { ...version(p),trackId:t.id,name:t.name,...window,notes:t.notes.filter(n=>n.start>=window.start&&n.start<window.end).map(n=>({ ...n,note:noteName(n.pitch),bar:Math.floor(n.start/p.beatsPerBar)+1,offsetInBar:n.start%p.beatsPerBar })) };
    },
    async dryRun(input: ChangeInput) {
      validateChanges(input);const before=await host.read();expect(before,input);
      const after=applyOperations(before,copy(input.operations));
      return { label:input.label,dryRun:true,...summary(before,after),...version(before),proposedRevision:after.revision };
    },
    async batch(input: ChangeInput) {
      validateChanges(input);const before=await host.read();expect(before,input);
      const after=await host.write({ action:'batch',...version(before),operations:copy(input.operations) },`协作 · ${input.label}`);
      return { label:input.label,dryRun:false,...summary(before,after) };
    },
    async replaceNotes(input: VersionRef & Range & { trackId: string; notes: RelativeNote[]; label: string; dryRun?: boolean }) {
      const p=await host.read();expect(p,input);const t=track(p,input.trackId),window=range(p,input);
      if(!Array.isArray(input.notes))throw Error('notes 必须是数组，空数组表示仅清空所选范围');
      const notes=input.notes.map(n=>{
        if(!Number.isFinite(n.offset)||n.offset<0||n.offset>=window.end-window.start||!Number.isFinite(n.duration)||n.duration<=0||n.offset+n.duration>window.end-window.start)throw Error('新音符的 offset/duration 必须完整落在所选小节范围内，不会静默裁剪');
        return { ...(n.id ? { id:n.id } : {}),pitch:n.pitch,start:window.start+n.offset,duration:n.duration,velocity:n.velocity };
      });
      const ids=t.notes.filter(n=>n.start>=window.start&&n.start<window.end).map(n=>n.id);
      const operations: Operation[]=[{ type:'notes.remove',trackId:t.id,ids },{ type:'notes.add',trackId:t.id,notes }];
      return input.dryRun?api.dryRun({ ...input,operations }):api.batch({ ...input,operations });
    },
    async undo(ref: VersionRef) { const before=await host.read();expect(before,ref);const after=await host.write({ action:'undo',...ref },'协作 · 撤销上一轮');return summary(before,after); },
    async redo(ref: VersionRef) { const before=await host.read();expect(before,ref);const after=await host.write({ action:'redo',...ref },'协作 · 重做上一轮');return summary(before,after); },
    async transport(input: VersionRef & { type: 'play'|'pause'|'stop'|'seek'; bar?: number; beat?: number }) {
      const p=await host.read();expect(p,input);
      if(!['play','pause','stop','seek'].includes(input.type))throw Error('未知播放命令');
      if(input.bar!==undefined&&input.beat!==undefined)throw Error('bar 和 beat 只能使用一个');
      const beat=input.bar!==undefined?(input.bar-1)*p.beatsPerBar:input.beat;
      if(input.type==='seek'&&beat===undefined)throw Error('seek 需要 bar 或 beat');
      if(beat!==undefined&&(!Number.isFinite(beat)||beat<0||beat>=p.bars*p.beatsPerBar))throw Error('播放位置超出当前工程');
      if(input.type==='play'&&!host.status().unlocked)throw Error('AUDIO_LOCKED: 请先在当前网页点击播放或试听键盘以解锁音频');
      if(beat!==undefined && input.type!=='seek')await host.transport('seek',beat);
      await host.transport(input.type,beat);
      return copy(host.status());
    },
    async view(input: { type: 'arrangement'|'instrument'|'effects'|'piano'; trackId?: string; bar?: number }) {
      const p=await host.read();
      if(!['arrangement','instrument','effects','piano'].includes(input.type))throw Error('未知页面');
      if(input.type!=='arrangement')track(p,input.trackId!);
      const beat=input.bar===undefined?0:(input.bar-1)*p.beatsPerBar;
      if(!Number.isFinite(beat)||beat<0||beat>=p.bars*p.beatsPerBar)throw Error('乐段位置超出当前工程');
      host.view(input.type,input.trackId,beat);return { ...version(p),...input };
    },
    async exportProject() { const p=await host.read();return { ...version(p),filename:`${p.id}_r${p.revision}.operitmusic.json`,json:JSON.stringify(p,null,2) }; },
  };
  return Object.freeze(api);
}
export type LiveControl = ReturnType<typeof createLiveControl>;

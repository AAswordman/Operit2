"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.COLORS = exports.LIMITS = void 0;
exports.uid = uid;
exports.copy = copy;
exports.emptyProject = emptyProject;
exports.noteName = noteName;
exports.audible = audible;
exports.eventBeat = eventBeat;
exports.LIMITS = { tracks: 24, notes: 6000, totalNotes: 16000, bars: 256, voices: 64, projects: 30, renderSeconds: 240, effects: 6, jsonBytes: 8000000 };
exports.COLORS = ["#b8f36b", "#b6a0ff", "#ffb56b", "#65dbe5", "#ed8ac5", "#879eff", "#dfd27d", "#88d5ae"];
function uid(prefix = "id") { return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 9)}`; }
function copy(value) { return JSON.parse(JSON.stringify(value)); }
function emptyProject(name = "Untitled session") {
    const now = new Date().toISOString();
    return { format: "operit-music", version: 1, id: uid("project"), revision: 0, name, bpm: 112, beatsPerBar: 4, bars: 8, swing: 0, masterGain: 0.72, loop: { enabled: true, start: 0, end: 32 }, tracks: [], sections: [], createdAt: now, updatedAt: now };
}
function noteName(pitch) { return ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"][pitch % 12] + (Math.floor(pitch / 12) - 1); }
function audible(track, project) { return !track.mute && (!project.tracks.some(t => t.solo) || track.solo); }
function eventBeat(note, swing) { return note.start + (Math.abs(note.start % 1 - 0.5) < 0.001 ? swing * 0.5 : 0); }

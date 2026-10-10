"use strict";
// Public client copied by dependency packages; keep this file self-contained.
Object.defineProperty(exports, "__esModule", { value: true });
exports.characterCards = void 0;
/** Calls an explicitly registered method on the character package's dependency boundary. */
function call(operation, payload) {
    return ToolPkg.callDependency("com.operit.character_cards", operation, payload);
}
/** Exposes typed domain methods to packages declaring a dependency on character cards. */
exports.characterCards = {
    chat: {
        configuration: {
            /** Resolves this chat's stored configuration without changing its selection. */
            resolve: (payload) => call("chat.configuration.resolve", payload),
            binding: {
                /** Requires one existing chat selection. */
                read: (payload) => call("chat.configuration.binding.read", payload),
                /** Commits an explicit selection through the sole authoritative service. */
                write: (payload) => call("chat.configuration.binding.write", payload),
                /** Deletes an existing selection without inventing another actor. */
                delete: (payload) => call("chat.configuration.binding.delete", payload),
            },
        },
    },
    /** Reads the editor directory. */
    snapshot: () => call("snapshot", {}),
    characters: {
        /** Lists complete characters. */
        list: () => call("character.list", {}),
        /** Reads a character. */
        get: (payload) => call("character.get", payload),
        /** Creates a character. */
        create: (payload) => call("character.create", payload),
        /** Applies a character patch. */
        update: (payload) => call("character.update", payload),
        /** Deletes a character. */
        delete: (payload) => call("character.delete", payload),
        /** Activates a character through the domain service. */
        setActive: (payload) => call("character.setActive", payload),
        /** Combines the character prompt and selected tags. */
        combine: (payload) => call("character.combine", payload),
        /** Resets the built-in character. */
        resetDefault: () => call("character.resetDefault", {}),
        /** Exports an explicitly selected character format. */
        export: (payload) => call("character.export", payload),
        /** Imports an explicitly selected character format. */
        import: (payload) => call("character.import", payload),
        /** Exports all characters and prompt tags. */
        exportBackup: () => call("character.exportBackup", {}),
        /** Imports a character and tag backup. */
        importBackup: (payload) => call("character.importBackup", payload),
    },
    groups: {
        /** Lists complete groups. */
        list: () => call("group.list", {}),
        /** Reads a group. */
        get: (payload) => call("group.get", payload),
        /** Creates a group. */
        create: (payload) => call("group.create", payload),
        /** Applies a group patch. */
        update: (payload) => call("group.update", payload),
        /** Deletes a group. */
        delete: (payload) => call("group.delete", payload),
        /** Activates a group through the domain service. */
        setActive: (payload) => call("group.setActive", payload),
        /** Duplicates a group. */
        duplicate: (payload) => call("group.duplicate", payload),
        /** Exports one native group. */
        export: (payload) => call("group.export", payload),
        /** Imports one native group. */
        import: (payload) => call("group.import", payload),
        /** Exports all groups. */
        exportBackup: () => call("group.exportBackup", {}),
        /** Imports a group backup. */
        importBackup: (payload) => call("group.importBackup", payload),
    },
    activePrompt: {
        /** Reads the active prompt. */
        get: () => call("activePrompt.get", {}),
        /** Activates a character card. */
        setCard: (payload) => call("activePrompt.setCard", payload),
        /** Activates a character group. */
        setGroup: (payload) => call("activePrompt.setGroup", payload),
        /** Applies the chat binding through the manager. */
        activateForChat: (payload) => call("activePrompt.activateForChat", payload),
        /** Resolves the active sending card. */
        resolvedCard: () => call("activePrompt.resolvedCard", {}),
    },
    tags: {
        /** Lists prompt tags. */
        list: () => call("tag.list", {}),
        /** Reads a prompt tag. */
        get: (payload) => call("tag.get", payload),
        /** Creates a prompt tag. */
        create: (payload) => call("tag.create", payload),
        /** Applies a prompt tag patch. */
        update: (payload) => call("tag.update", payload),
        /** Deletes a prompt tag. */
        delete: (payload) => call("tag.delete", payload),
    },
    memory: {
        /** Queries participant-bound memories using provider-owned snapshots and document matching. */
        query: (payload) => ToolPkg.callDependency("com.operit.character_cards", "memory.query", payload),
        chats: {
            /** Lists actual generic chat summaries selected by persisted plugin bindings. */
            list: (payload) => call("memory.chat.list", payload),
            /** Applies genuine functional MEMORY extraction to the selected real chat. */
            update: (payload) => call("memory.chat.update", payload),
        },
        /** Categorizes full root memories using the configured functional MEMORY model. */
        categorize: (payload) => call("memory.categorize", payload),
        rebuild: {
            /** Persists a real rebuild plan for interval execution. */
            start: (payload) => call("memory.rebuild.start", payload),
            /** Reads actual durable progress counters. */
            progress: (payload) => call("memory.rebuild.progress", payload),
            /** Cancels a genuine active plan before another source window. */
            cancel: (payload) => call("memory.rebuild.cancel", payload),
        },
        embeddings: {
            /** Recomputes real provider vectors over every complete node and chunk. */
            rebuild: (payload) => call("memory.embeddings.rebuild", payload),
        },
        candidates: {
            /** Enqueues an actual finalized reply or explicit selected-user message. */
            enqueue: (payload) => call("memory.candidate.enqueue", payload),
        },
        shared: {
            /** Lists shared memory libraries. */
            list: () => call("memory.shared.list", {}),
            /** Creates a shared memory library. */
            create: (payload) => call("memory.shared.create", payload),
            /** Renames a shared memory library. */
            rename: (payload) => call("memory.shared.rename", payload),
            /** Deletes a library and cleans character mounts. */
            delete: (payload) => call("memory.shared.delete", payload),
        },
        /** Mounts a shared library with explicit permissions. */
        mount: (payload) => call("memory.mount", payload),
        /** Removes a shared library mount. */
        unmount: (payload) => call("memory.unmount", payload),
        user: {
            /** Reads the owner's USER.md. */
            read: (payload) => call("memory.user.read", payload),
            /** Writes the owner's USER.md. */
            write: (payload) => call("memory.user.write", payload),
            /** Reads the real plugin-owned USER.md path accepted by Files. */
            path: (payload) => call("memory.user.path", payload),
        },
        /** Resolves a character's actual bound memory owner. */
        resolveOwner: (payload) => call("memory.resolveOwner", payload),
        settings: {
            /** Reads owner-scoped memory settings. */
            read: (payload) => call("memory.settings.read", payload),
            /** Writes owner-scoped memory settings. */
            write: (payload) => call("memory.settings.write", payload),
        },
        searchConfig: {
            /** Reads the search scoring configuration. */
            read: (payload) => call("memory.searchConfig.read", payload),
            /** Writes the search scoring configuration. */
            write: (payload) => call("memory.searchConfig.write", payload),
        },
        /** Reads a memory graph. */
        graph: (payload) => call("memory.graph", payload),
        /** Lists memory records. */
        list: (payload) => call("memory.list", payload),
        /** Searches full records with explicit filters through the sole service and its durable embedding cache. */
        searchWithOptions: (payload) => call("memory.searchWithOptions", payload),
        /** Searches memory records. */
        search: (payload) => call("memory.search", payload),
        /** Reads a memory record by title. */
        get: (payload) => call("memory.get", payload),
        /** Creates a memory record. */
        create: (payload) => call("memory.create", payload),
        /** Applies a memory record patch. */
        update: (payload) => call("memory.update", payload),
        /** Deletes a memory record by its stable string ID. */
        delete: (payload) => call("memory.delete", payload),
        /** Moves memory records to a folder. */
        move: (payload) => call("memory.move", payload),
        links: {
            /** Creates a memory relationship. */
            create: (payload) => call("memory.link.create", payload),
            /** Applies a memory relationship patch. */
            update: (payload) => call("memory.link.update", payload),
            /** Deletes a memory relationship. */
            delete: (payload) => call("memory.link.delete", payload),
        },
        /** Exports a repository memory backup. */
        export: (payload) => call("memory.export", payload),
        /** Imports a backup using the explicitly selected strategy. */
        import: (payload) => call("memory.import", payload),
    },
};

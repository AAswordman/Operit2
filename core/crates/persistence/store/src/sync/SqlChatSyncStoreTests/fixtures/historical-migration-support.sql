-- Unchanged historical tables required for full migrations from versions 22 and 23.
CREATE TABLE usage_request_records (
            id TEXT PRIMARY KEY NOT NULL,
            createdAtMs INTEGER NOT NULL,
            providerModel TEXT NOT NULL,
            provider TEXT NOT NULL,
            modelName TEXT NOT NULL,
            functionType TEXT NOT NULL,
            source TEXT NOT NULL,
            chatId TEXT,
            inputTokens INTEGER NOT NULL,
            outputTokens INTEGER NOT NULL,
            cachedInputTokens INTEGER NOT NULL
        );

CREATE TABLE sync_sql_chat_rows (
            opId TEXT NOT NULL,
            id TEXT NOT NULL,
            title TEXT NOT NULL,
            createdAt INTEGER NOT NULL,
            updatedAt INTEGER NOT NULL,
            inputTokens INTEGER NOT NULL,
            outputTokens INTEGER NOT NULL,
            currentWindowSize INTEGER NOT NULL,
            "group" TEXT,
            displayOrder INTEGER NOT NULL,
            workspaceId TEXT,
            workspaceEnv TEXT,
            parentChatId TEXT,
            characterCardName TEXT,
            characterGroupId TEXT,
            locked INTEGER NOT NULL,
            pinned INTEGER NOT NULL,
            PRIMARY KEY(opId, id),
            FOREIGN KEY(opId) REFERENCES sync_sql_operations(opId) ON DELETE CASCADE
        );



ALTER TABLE chats ADD COLUMN characterCardName TEXT;
ALTER TABLE chats ADD COLUMN characterGroupId TEXT;

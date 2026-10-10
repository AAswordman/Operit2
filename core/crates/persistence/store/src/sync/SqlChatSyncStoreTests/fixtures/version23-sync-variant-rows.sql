-- Historical sync variant table omitted by the version-23 message fixture.
CREATE TABLE sync_sql_message_variant_rows (
            opId TEXT NOT NULL,
            chatId TEXT NOT NULL,
            messageTimestamp INTEGER NOT NULL,
            variantIndex INTEGER NOT NULL,
            roleName TEXT NOT NULL,
            provider TEXT NOT NULL,
            modelName TEXT NOT NULL,
            inputTokens INTEGER NOT NULL,
            outputTokens INTEGER NOT NULL,
            cachedInputTokens INTEGER NOT NULL,
            sentAt INTEGER NOT NULL,
            outputDurationMs INTEGER NOT NULL,
            waitDurationMs INTEGER NOT NULL,
            completedAt INTEGER NOT NULL,
            PRIMARY KEY(opId, chatId, messageTimestamp, variantIndex),
            FOREIGN KEY(opId) REFERENCES sync_sql_operations(opId) ON DELETE CASCADE
        );

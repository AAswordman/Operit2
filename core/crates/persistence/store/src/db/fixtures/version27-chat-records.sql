-- Exact version-27 production schema with populated legacy fields for physical migration tests.

        CREATE TABLE chats (
            id TEXT PRIMARY KEY NOT NULL,
            title TEXT NOT NULL,
            createdAt INTEGER NOT NULL,
            updatedAt INTEGER NOT NULL,
            inputTokens INTEGER NOT NULL DEFAULT 0,
            outputTokens INTEGER NOT NULL DEFAULT 0,
            currentWindowSize INTEGER NOT NULL DEFAULT 0,
            "group" TEXT,
            displayOrder INTEGER NOT NULL DEFAULT 0,
            workspaceId TEXT,
            workspaceEnv TEXT,
            parentChatId TEXT,
            characterCardName TEXT,
            characterGroupId TEXT,
            locked INTEGER NOT NULL DEFAULT 0,
            pinned INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE messages (
            messageId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            chatId TEXT NOT NULL,
            sender TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            orderIndex INTEGER NOT NULL,
            roleName TEXT NOT NULL DEFAULT '',
            selectedVariantIndex INTEGER NOT NULL DEFAULT 0,
            provider TEXT NOT NULL DEFAULT '',
            modelName TEXT NOT NULL DEFAULT '',
            inputTokens INTEGER NOT NULL DEFAULT 0,
            outputTokens INTEGER NOT NULL DEFAULT 0,
            cachedInputTokens INTEGER NOT NULL DEFAULT 0,
            sentAt INTEGER NOT NULL DEFAULT 0,
            outputDurationMs INTEGER NOT NULL DEFAULT 0,
            waitDurationMs INTEGER NOT NULL DEFAULT 0,
            completedAt INTEGER NOT NULL DEFAULT 0,
            completedExecutionGeneration INTEGER NOT NULL DEFAULT 0,
            displayMode TEXT NOT NULL DEFAULT 'NORMAL',
            isFavorite INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(chatId) REFERENCES chats(id) ON DELETE CASCADE
        );

        CREATE TABLE message_variants (
            variantId INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            chatId TEXT NOT NULL,
            messageTimestamp INTEGER NOT NULL,
            variantIndex INTEGER NOT NULL,
            roleName TEXT NOT NULL DEFAULT '',
            provider TEXT NOT NULL DEFAULT '',
            modelName TEXT NOT NULL DEFAULT '',
            inputTokens INTEGER NOT NULL DEFAULT 0,
            outputTokens INTEGER NOT NULL DEFAULT 0,
            cachedInputTokens INTEGER NOT NULL DEFAULT 0,
            sentAt INTEGER NOT NULL DEFAULT 0,
            outputDurationMs INTEGER NOT NULL DEFAULT 0,
            waitDurationMs INTEGER NOT NULL DEFAULT 0,
            completedAt INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(chatId) REFERENCES chats(id) ON DELETE CASCADE
        );

        CREATE UNIQUE INDEX IF NOT EXISTS index_messages_chatId_timestamp
            ON messages(chatId, timestamp);
        CREATE INDEX IF NOT EXISTS index_messages_chatId_orderIndex
            ON messages(chatId, orderIndex);
        CREATE INDEX IF NOT EXISTS index_message_variants_chatId_messageTimestamp
            ON message_variants(chatId, messageTimestamp);
        CREATE UNIQUE INDEX IF NOT EXISTS index_message_variants_chatId_messageTimestamp_variantIndex
            ON message_variants(chatId, messageTimestamp, variantIndex);

        CREATE TABLE message_parts (
            chatId TEXT NOT NULL,
            messageTimestamp INTEGER NOT NULL,
            variantIndex INTEGER NOT NULL,
            partId TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            kind TEXT NOT NULL,
            content TEXT NOT NULL,
            toolCallId TEXT,
            toolName TEXT,
            attributesJson TEXT NOT NULL,
            PRIMARY KEY(chatId, messageTimestamp, variantIndex, partId),
            UNIQUE(chatId, messageTimestamp, variantIndex, sequence),
            FOREIGN KEY(chatId) REFERENCES chats(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS index_message_parts_chatId_messageTimestamp
            ON message_parts(chatId, messageTimestamp, variantIndex, sequence);


        CREATE TABLE IF NOT EXISTS usage_request_records (
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
        CREATE INDEX IF NOT EXISTS index_usage_request_records_createdAtMs
            ON usage_request_records(createdAtMs);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_providerModel
            ON usage_request_records(providerModel);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_provider
            ON usage_request_records(provider);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_modelName
            ON usage_request_records(modelName);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_functionType
            ON usage_request_records(functionType);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_source
            ON usage_request_records(source);
        CREATE INDEX IF NOT EXISTS index_usage_request_records_chatId
            ON usage_request_records(chatId);


        CREATE TABLE IF NOT EXISTS token_usage_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            importKey TEXT,
            occurredAtMs INTEGER,
            configId TEXT NOT NULL,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            requestCount INTEGER NOT NULL DEFAULT 1,
            uncachedInputTokens INTEGER,
            cachedInputTokens INTEGER,
            cacheWriteTokens INTEGER,
            totalInputTokens INTEGER,
            outputTokens INTEGER
        );
        CREATE UNIQUE INDEX IF NOT EXISTS index_token_usage_records_importKey
            ON token_usage_records(importKey);
        CREATE INDEX IF NOT EXISTS index_token_usage_records_occurredAtMs
            ON token_usage_records(occurredAtMs);
        CREATE INDEX IF NOT EXISTS index_token_usage_records_provider_model_configId_occurredAtMs
            ON token_usage_records(provider, model, configId, occurredAtMs);
        CREATE TABLE IF NOT EXISTS token_stats_models (
            configId TEXT NOT NULL,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            billingMode TEXT,
            currency TEXT,
            inputPricePerMillion REAL,
            cachedInputPricePerMillion REAL,
            cacheWritePricePerMillion REAL,
            outputPricePerMillion REAL,
            pricePerRequest REAL,
            PRIMARY KEY(configId, provider, model)
        );


        CREATE TABLE IF NOT EXISTS sync_sql_clocks (
            originDeviceId TEXT PRIMARY KEY NOT NULL,
            sequence INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sync_sql_operations (
            opId TEXT PRIMARY KEY NOT NULL,
            originDeviceId TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            domain TEXT NOT NULL,
            entityType TEXT NOT NULL,
            entityId TEXT NOT NULL,
            operation TEXT NOT NULL,
            semantics TEXT NOT NULL,
            createdAt INTEGER NOT NULL,
            schemaVersion INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS index_sync_sql_operations_origin_sequence
            ON sync_sql_operations(originDeviceId, sequence);
        CREATE INDEX IF NOT EXISTS index_sync_sql_operations_createdAt
            ON sync_sql_operations(createdAt);

        CREATE TABLE IF NOT EXISTS sync_sql_chat_rows (
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

        CREATE TABLE IF NOT EXISTS sync_sql_message_rows (
            opId TEXT NOT NULL,
            chatId TEXT NOT NULL,
            sender TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            orderIndex INTEGER NOT NULL,
            roleName TEXT NOT NULL,
            selectedVariantIndex INTEGER NOT NULL,
            provider TEXT NOT NULL,
            modelName TEXT NOT NULL,
            inputTokens INTEGER NOT NULL,
            outputTokens INTEGER NOT NULL,
            cachedInputTokens INTEGER NOT NULL,
            sentAt INTEGER NOT NULL,
            outputDurationMs INTEGER NOT NULL,
            waitDurationMs INTEGER NOT NULL,
            completedAt INTEGER NOT NULL,
            completedExecutionGeneration INTEGER NOT NULL,
            displayMode TEXT NOT NULL,
            isFavorite INTEGER NOT NULL,
            PRIMARY KEY(opId, chatId, timestamp),
            FOREIGN KEY(opId) REFERENCES sync_sql_operations(opId) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS sync_sql_message_variant_rows (
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

        CREATE TABLE IF NOT EXISTS sync_sql_message_part_rows (
            opId TEXT NOT NULL,
            chatId TEXT NOT NULL,
            messageTimestamp INTEGER NOT NULL,
            variantIndex INTEGER NOT NULL,
            partId TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            kind TEXT NOT NULL,
            content TEXT NOT NULL,
            toolCallId TEXT,
            toolName TEXT,
            attributesJson TEXT NOT NULL,
            PRIMARY KEY(opId, chatId, messageTimestamp, variantIndex, partId),
            UNIQUE(opId, chatId, messageTimestamp, variantIndex, sequence),
            FOREIGN KEY(opId) REFERENCES sync_sql_operations(opId) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS sync_sql_deletions (
            opId TEXT NOT NULL,
            ordinal INTEGER NOT NULL,
            tableName TEXT NOT NULL,
            chatId TEXT NOT NULL,
            messageTimestamp INTEGER,
            variantIndex INTEGER,
            PRIMARY KEY(opId, ordinal),
            FOREIGN KEY(opId) REFERENCES sync_sql_operations(opId) ON DELETE CASCADE
        );


INSERT INTO chats (id,title,createdAt,updatedAt,inputTokens,outputTokens,currentWindowSize,"group",displayOrder,workspaceId,workspaceEnv,parentChatId,characterCardName,characterGroupId,locked,pinned)
VALUES ('v27-chat','Kept title',100,200,3,5,8,'legacy-sidebar',12,'workspace-kept','workspace-env-kept','parent-kept','legacy-card','legacy-role-group',1,1);
INSERT INTO messages (chatId,sender,timestamp,orderIndex,roleName,selectedVariantIndex,provider,modelName,inputTokens,outputTokens,cachedInputTokens,sentAt,outputDurationMs,waitDurationMs,completedAt,completedExecutionGeneration,displayMode,isFavorite)
VALUES ('v27-chat','ai',42,7,'Legacy speaker',2,'provider-kept','model-kept',11,13,17,50,19,23,90,29,'NORMAL',1);
INSERT INTO message_variants (chatId,messageTimestamp,variantIndex,roleName,provider,modelName,inputTokens,outputTokens,cachedInputTokens,sentAt,outputDurationMs,waitDurationMs,completedAt)
VALUES ('v27-chat',42,1,'Variant one','provider-1','model-1',31,37,41,51,43,47,91), ('v27-chat',42,2,'Variant two','provider-2','model-2',53,59,61,52,67,71,92);
INSERT INTO message_parts (chatId,messageTimestamp,variantIndex,partId,sequence,kind,content,toolCallId,toolName,attributesJson)
VALUES ('v27-chat',42,0,'part-0',0,'markdown','base-kept',NULL,NULL,'{"base":"true"}'),('v27-chat',42,1,'part-0',0,'markdown','variant-one-kept','call-kept','tool-kept','{"variant":"1"}'),('v27-chat',42,2,'part-0',0,'markdown','variant-two-kept',NULL,NULL,'{"variant":"2"}');
INSERT INTO sync_sql_operations (opId,originDeviceId,sequence,domain,entityType,entityId,operation,semantics,createdAt,schemaVersion)
VALUES ('v27:1','v27',1,'chat','chat','v27-chat','upsert','transaction',100,6), ('v27:2','v27',2,'usage','request','usage-kept','upsert','transaction',101,6);
INSERT INTO sync_sql_clocks (originDeviceId,sequence) VALUES ('v27',2);
INSERT INTO sync_sql_chat_rows (opId,id,title,createdAt,updatedAt,inputTokens,outputTokens,currentWindowSize,"group",displayOrder,workspaceId,workspaceEnv,parentChatId,characterCardName,characterGroupId,locked,pinned)
SELECT 'v27:1',id,title,createdAt,updatedAt,inputTokens,outputTokens,currentWindowSize,"group",displayOrder,workspaceId,workspaceEnv,parentChatId,characterCardName,characterGroupId,locked,pinned FROM chats;
INSERT INTO sync_sql_message_rows (opId,chatId,sender,timestamp,orderIndex,roleName,selectedVariantIndex,provider,modelName,inputTokens,outputTokens,cachedInputTokens,sentAt,outputDurationMs,waitDurationMs,completedAt,completedExecutionGeneration,displayMode,isFavorite)
SELECT 'v27:1',chatId,sender,timestamp,orderIndex,roleName,selectedVariantIndex,provider,modelName,inputTokens,outputTokens,cachedInputTokens,sentAt,outputDurationMs,waitDurationMs,completedAt,completedExecutionGeneration,displayMode,isFavorite FROM messages;
INSERT INTO sync_sql_message_variant_rows (opId,chatId,messageTimestamp,variantIndex,roleName,provider,modelName,inputTokens,outputTokens,cachedInputTokens,sentAt,outputDurationMs,waitDurationMs,completedAt)
SELECT 'v27:1',chatId,messageTimestamp,variantIndex,roleName,provider,modelName,inputTokens,outputTokens,cachedInputTokens,sentAt,outputDurationMs,waitDurationMs,completedAt FROM message_variants;
INSERT INTO sync_sql_message_part_rows (opId,chatId,messageTimestamp,variantIndex,partId,sequence,kind,content,toolCallId,toolName,attributesJson)
SELECT 'v27:1',chatId,messageTimestamp,variantIndex,partId,sequence,kind,content,toolCallId,toolName,attributesJson FROM message_parts;
INSERT INTO sync_sql_deletions (opId,ordinal,tableName,chatId,messageTimestamp,variantIndex)
VALUES ('v27:1',0,'message','previously-deleted',39,0);
INSERT INTO usage_request_records(id,createdAtMs,providerModel,provider,modelName,functionType,source,chatId,inputTokens,outputTokens,cachedInputTokens)
VALUES('usage-kept',111,'provider:model','provider','model','CHAT','request','v27-chat',7,11,2);
INSERT INTO token_usage_records(importKey,occurredAtMs,configId,provider,model,requestCount,uncachedInputTokens,cachedInputTokens,cacheWriteTokens,totalInputTokens,outputTokens)
VALUES('import-kept',112,'config-kept','provider','model',3,5,7,11,23,13);
INSERT INTO token_stats_models(configId,provider,model,billingMode,currency,inputPricePerMillion,cachedInputPricePerMillion,cacheWritePricePerMillion,outputPricePerMillion,pricePerRequest)
VALUES('config-kept','provider','model','tokens','USD',1.5,0.5,2.0,3.5,0.1);
PRAGMA user_version = 27;

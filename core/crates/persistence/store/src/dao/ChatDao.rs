use crate::sqliteParams;
use crate::PluginExtensions::{decodePluginExtensions, encodePluginExtensions};
use crate::PreferencesDataStore::StateFlow;
use crate::SqliteStore::{
    toSqliteValue, SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError, SqliteValue,
};

use operit_model::ChatEntity::ChatEntity;

#[derive(Clone)]
pub struct ChatDao {
    store: SqliteStore,
}

impl ChatDao {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }

    pub fn getAllChats(&self) -> Result<StateFlow<Vec<ChatEntity>>, SqliteStoreError> {
        self.observeChats(
            "SELECT * FROM chats ORDER BY pinned DESC, displayOrder ASC".to_string(),
            Vec::new(),
        )
    }

    pub fn getTotalChatCount(&self) -> Result<i32, SqliteStoreError> {
        self.store
            .queryScalar("SELECT COUNT(*) FROM chats", sqliteParams![])
    }

    pub fn getAllChatsDirectly(&self) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChats(
            "SELECT * FROM chats ORDER BY pinned DESC, displayOrder ASC",
            sqliteParams![],
        )
    }

    pub fn getChatById(&self, chatId: &str) -> Result<Option<ChatEntity>, SqliteStoreError> {
        self.store
            .queryOne("SELECT * FROM chats WHERE id = ?1", sqliteParams![chatId])?
            .map(|row| mapChatEntity(&row))
            .transpose()
    }

    /// Inserts one metadata record with validated plugin-owned namespaces.
    pub fn insertChat(&self, chat: ChatEntity) -> Result<(), SqliteStoreError> {
        self.store
            .execute(chatInsertSql(), chatInsertParams(&chat)?)?;
        self.store.notifyInvalidated()
    }

    pub fn deleteChat(&self, chatId: &str) -> Result<(), SqliteStoreError> {
        self.execute("DELETE FROM chats WHERE id = ?1", sqliteParams![chatId])
    }

    pub fn updateChatMetadata(
        &self,
        chatId: &str,
        title: String,
        timestamp: i64,
        inputTokens: i64,
        outputTokens: i64,
        currentWindowSize: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET updatedAt = ?2, title = ?3, inputTokens = ?4, outputTokens = ?5, currentWindowSize = ?6 WHERE id = ?1",
            sqliteParams![chatId, timestamp, title, inputTokens, outputTokens, currentWindowSize],
        )
    }

    pub fn updateChatTitle(
        &self,
        chatId: &str,
        title: String,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET title = ?2, updatedAt = ?3 WHERE id = ?1",
            sqliteParams![chatId, title, timestamp],
        )
    }

    pub fn updateChatWorkspaceId(
        &self,
        chatId: &str,
        workspaceId: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET workspaceId = ?2, updatedAt = ?3 WHERE id = ?1",
            sqliteParams![chatId, workspaceId, timestamp],
        )
    }

    pub fn updateChatTitleAndWorkspaceId(
        &self,
        chatId: &str,
        title: String,
        workspaceId: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET title = ?2, workspaceId = ?3, updatedAt = ?4 WHERE id = ?1",
            sqliteParams![chatId, title, workspaceId, timestamp],
        )
    }

    pub fn updateChatLocked(
        &self,
        chatId: &str,
        locked: bool,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET locked = ?2, updatedAt = ?3 WHERE id = ?1",
            sqliteParams![chatId, locked, timestamp],
        )
    }

    pub fn updateChatPinned(
        &self,
        chatId: &str,
        pinned: bool,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET pinned = ?2, updatedAt = ?3 WHERE id = ?1",
            sqliteParams![chatId, pinned, timestamp],
        )
    }

    /// Promotes an active chat without changing its pin, workspace, or plugin namespaces.
    pub fn moveChatToFront(&self, chatId: &str, timestamp: i64) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET displayOrder = MIN(-?2, (SELECT MIN(displayOrder) FROM chats) - 1), updatedAt = ?2 WHERE id = ?1",
            sqliteParams![chatId, timestamp],
        )
    }

    /// Updates only the ordering of one existing chat record.
    pub fn updateChatOrder(&self, chatId: &str, displayOrder: i64) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET displayOrder = ?2 WHERE id = ?1",
            sqliteParams![chatId, displayOrder],
        )
    }

    /// Updates host-owned metadata without replacing any plugin extension namespace.
    pub fn updateChats(&self, chats: Vec<ChatEntity>) -> Result<(), SqliteStoreError> {
        self.store.transaction(|transaction| {
            for chat in chats {
                transaction.execute(
                    "UPDATE chats SET title = ?2, createdAt = ?3, updatedAt = ?4, inputTokens = ?5, outputTokens = ?6, currentWindowSize = ?7, displayOrder = ?8, workspaceId = ?9, parentChatId = ?10, locked = ?11, pinned = ?12, \"group\" = ?13 WHERE id = ?1",
                    sqliteParams![chat.id, chat.title, chat.createdAt, chat.updatedAt, chat.inputTokens, chat.outputTokens, chat.currentWindowSize, chat.displayOrder, chat.workspaceId, chat.parentChatId, chat.locked, chat.pinned, chat.group],
                )?;
            }
            Ok(())
        })?;
        self.store.notifyInvalidated()
    }

    pub fn getBranchesByParentId(
        &self,
        parentChatId: &str,
    ) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChatsWithOne(
            "SELECT * FROM chats WHERE parentChatId = ?1 ORDER BY pinned DESC, displayOrder ASC",
            parentChatId,
        )
    }

    pub fn getBranchesByParentIdFlow(
        &self,
        parentChatId: &str,
    ) -> Result<StateFlow<Vec<ChatEntity>>, SqliteStoreError> {
        self.observeChats(
            "SELECT * FROM chats WHERE parentChatId = ?1 ORDER BY pinned DESC, displayOrder ASC"
                .to_string(),
            vec![parentChatId.to_string()],
        )
    }

    pub fn getMainChats(&self) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChats(
            "SELECT * FROM chats WHERE parentChatId IS NULL ORDER BY pinned DESC, displayOrder ASC",
            sqliteParams![],
        )
    }

    pub fn getMainChatsFlow(&self) -> Result<StateFlow<Vec<ChatEntity>>, SqliteStoreError> {
        self.observeChats(
            "SELECT * FROM chats WHERE parentChatId IS NULL ORDER BY pinned DESC, displayOrder ASC"
                .to_string(),
            Vec::new(),
        )
    }

    fn execute(&self, sql: &str, params: Vec<SqliteValue>) -> Result<(), SqliteStoreError> {
        self.store.execute(sql, params)?;
        self.store.notifyInvalidated()
    }

    fn execute_count(&self, sql: &str, params: Vec<SqliteValue>) -> Result<i32, SqliteStoreError> {
        let count = self.store.execute(sql, params)? as i32;
        self.store.notifyInvalidated()?;
        Ok(count)
    }

    fn selectChats(
        &self,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.store
            .queryRows(sql, params)?
            .into_iter()
            .map(|row| mapChatEntity(&row))
            .collect()
    }

    fn selectChatsWithOne(
        &self,
        sql: &str,
        value: &str,
    ) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChats(sql, sqliteParams![value])
    }

    fn execute_for_chat_ids(
        &self,
        sqlPrefix: &str,
        chatIds: Vec<String>,
        mut leadingParams: Vec<SqliteValue>,
    ) -> Result<i32, SqliteStoreError> {
        let placeholders = chatIds.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("{sqlPrefix} ({placeholders})");
        for chatId in &chatIds {
            leadingParams.push(toSqliteValue(chatId));
        }
        let count = self.store.execute(&sql, leadingParams)? as i32;
        self.store.notifyInvalidated()?;
        Ok(count)
    }

    fn observeChats(
        &self,
        sql: String,
        values: Vec<String>,
    ) -> Result<StateFlow<Vec<ChatEntity>>, SqliteStoreError> {
        let stateFlow = StateFlow::new(self.selectChatsByValues(&sql, &values)?);
        let chatDao = self.clone();
        let stateFlowForObserver = stateFlow.clone();
        self.store.addInvalidationObserver(move || {
            stateFlowForObserver.set_value(chatDao.selectChatsByValues(&sql, &values)?);
            Ok(())
        })?;
        Ok(stateFlow)
    }

    fn selectChatsByValues(
        &self,
        sql: &str,
        values: &[String],
    ) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.store
            .queryRows(sql, values.iter().map(toSqliteValue).collect::<Vec<_>>())?
            .into_iter()
            .map(|row| mapChatEntity(&row))
            .collect()
    }
}

/// Decodes persisted host metadata and validated generic extension objects.
pub fn mapChatEntity(row: &SqliteRow) -> Result<ChatEntity, SqliteStoreError> {
    Ok(ChatEntity {
        id: row.get("id")?,
        title: row.get("title")?,
        group: row.get("group")?,
        pluginExtensions: decodePluginExtensions(&row.get::<_, String>("pluginExtensions")?)?,
        createdAt: row.get("createdAt")?,
        updatedAt: row.get("updatedAt")?,
        inputTokens: row.get("inputTokens")?,
        outputTokens: row.get("outputTokens")?,
        currentWindowSize: row.get("currentWindowSize")?,
        displayOrder: row.get("displayOrder")?,
        workspaceId: row.get("workspaceId")?,
        parentChatId: row.get("parentChatId")?,
        locked: row.get("locked")?,
        pinned: row.get("pinned")?,
    })
}

/// Returns the insert statement shared by the DAO and atomic chat draft commit.
pub(crate) fn chatInsertSql() -> &'static str {
    "INSERT INTO chats (id, title, createdAt, updatedAt, inputTokens, outputTokens, currentWindowSize, displayOrder, workspaceId, parentChatId, locked, pinned, pluginExtensions, \"group\") VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
}

/// Serializes a complete chat draft without interpreting extension values.
pub(crate) fn chatInsertParams(chat: &ChatEntity) -> Result<Vec<SqliteValue>, SqliteStoreError> {
    Ok(sqliteParams![
        chat.id,
        chat.title,
        chat.createdAt,
        chat.updatedAt,
        chat.inputTokens,
        chat.outputTokens,
        chat.currentWindowSize,
        chat.displayOrder,
        chat.workspaceId,
        chat.parentChatId,
        chat.locked,
        chat.pinned,
        encodePluginExtensions(&chat.pluginExtensions)?,
        chat.group
    ])
}

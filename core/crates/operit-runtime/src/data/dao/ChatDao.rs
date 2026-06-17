use operit_store::sqliteParams;
use operit_store::PreferencesDataStore::StateFlow;
use operit_store::SqliteStore::{
    toSqliteValue, SqliteRow, SqliteRowGet, SqliteStore, SqliteStoreError, SqliteValue,
};

use crate::data::model::CharacterCardChatStats::CharacterCardChatStats;
use crate::data::model::CharacterGroupChatStats::CharacterGroupChatStats;
use crate::data::model::ChatEntity::ChatEntity;

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

    pub fn insertChat(&self, chat: ChatEntity) -> Result<(), SqliteStoreError> {
        self.store.execute(
            r#"
                INSERT OR REPLACE INTO chats (
                    id, title, createdAt, updatedAt, inputTokens, outputTokens,
                    currentWindowSize, "group", displayOrder, workspace,
                    parentChatId, characterCardName, characterGroupId, locked, pinned
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                "#,
            sqliteParams![
                chat.id,
                chat.title,
                chat.createdAt,
                chat.updatedAt,
                chat.inputTokens,
                chat.outputTokens,
                chat.currentWindowSize,
                chat.group,
                chat.displayOrder,
                chat.workspace,
                chat.parentChatId,
                chat.characterCardName,
                chat.characterGroupId,
                chat.locked,
                chat.pinned,
            ],
        )?;
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
        inputTokens: i32,
        outputTokens: i32,
        currentWindowSize: i32,
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

    pub fn updateChatWorkspace(
        &self,
        chatId: &str,
        workspace: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET workspace = ?2, updatedAt = ?3 WHERE id = ?1",
            sqliteParams![chatId, workspace, timestamp],
        )
    }

    pub fn updateChatTitleAndWorkspace(
        &self,
        chatId: &str,
        title: String,
        workspace: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET title = ?2, workspace = ?3, updatedAt = ?4 WHERE id = ?1",
            sqliteParams![chatId, title, workspace, timestamp],
        )
    }

    pub fn updateChatGroup(
        &self,
        chatId: &str,
        group: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = ?2, updatedAt = ?3 WHERE id = ?1",
            sqliteParams![chatId, group, timestamp],
        )
    }

    pub fn updateChatCharacterCardName(
        &self,
        chatId: &str,
        characterCardName: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET characterCardName = ?2, characterGroupId = NULL, updatedAt = ?3 WHERE id = ?1",
            sqliteParams![chatId, characterCardName, timestamp],
        )
    }

    pub fn updateChatCharacterGroupId(
        &self,
        chatId: &str,
        characterGroupId: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET characterCardName = NULL, characterGroupId = ?2, updatedAt = ?3 WHERE id = ?1",
            sqliteParams![chatId, characterGroupId, timestamp],
        )
    }

    pub fn updateChatCharacterBinding(
        &self,
        chatId: &str,
        characterCardName: Option<String>,
        characterGroupId: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET characterCardName = ?2, characterGroupId = ?3, updatedAt = ?4 WHERE id = ?1",
            sqliteParams![chatId, characterCardName, characterGroupId, timestamp],
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

    pub fn updateChatOrderAndGroup(
        &self,
        chatId: &str,
        displayOrder: i64,
        group: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET displayOrder = ?2, \"group\" = ?3, updatedAt = ?4 WHERE id = ?1",
            sqliteParams![chatId, displayOrder, group, timestamp],
        )
    }

    pub fn updateChats(&self, chats: Vec<ChatEntity>) -> Result<(), SqliteStoreError> {
        self.store.transaction(|transaction| {
            for chat in chats {
                transaction.execute(
                    r#"
                    UPDATE chats
                    SET title = ?2,
                        createdAt = ?3,
                        updatedAt = ?4,
                        inputTokens = ?5,
                        outputTokens = ?6,
                        currentWindowSize = ?7,
                        "group" = ?8,
                        displayOrder = ?9,
                        workspace = ?10,
                        parentChatId = ?11,
                        characterCardName = ?12,
                        characterGroupId = ?13,
                        locked = ?14,
                        pinned = ?15
                    WHERE id = ?1
                    "#,
                    sqliteParams![
                        chat.id,
                        chat.title,
                        chat.createdAt,
                        chat.updatedAt,
                        chat.inputTokens,
                        chat.outputTokens,
                        chat.currentWindowSize,
                        chat.group,
                        chat.displayOrder,
                        chat.workspace,
                        chat.parentChatId,
                        chat.characterCardName,
                        chat.characterGroupId,
                        chat.locked,
                        chat.pinned,
                    ],
                )?;
            }
            Ok(())
        })?;
        self.store.notifyInvalidated()
    }

    pub fn updateGroupName(&self, oldName: &str, newName: &str) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = ?2 WHERE \"group\" = ?1",
            sqliteParams![oldName, newName],
        )
    }

    pub fn updateGroupNameForCharacter(
        &self,
        oldName: &str,
        newName: &str,
        characterCardName: &str,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = ?2 WHERE \"group\" = ?1 AND characterCardName = ?3",
            sqliteParams![oldName, newName, characterCardName],
        )
    }

    pub fn deleteChatsInGroup(&self, groupName: &str) -> Result<(), SqliteStoreError> {
        self.execute(
            "DELETE FROM chats WHERE \"group\" = ?1 AND locked = 0",
            sqliteParams![groupName],
        )
    }

    pub fn deleteChatsInGroupForCharacter(
        &self,
        groupName: &str,
        characterCardName: &str,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "DELETE FROM chats WHERE \"group\" = ?1 AND characterCardName = ?2 AND locked = 0",
            sqliteParams![groupName, characterCardName],
        )
    }

    pub fn removeGroupFromChats(
        &self,
        groupName: &str,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = NULL, updatedAt = ?2 WHERE \"group\" = ?1",
            sqliteParams![groupName, timestamp],
        )
    }

    pub fn removeGroupFromLockedChats(
        &self,
        groupName: &str,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = NULL, updatedAt = ?2 WHERE \"group\" = ?1 AND locked = 1",
            sqliteParams![groupName, timestamp],
        )
    }

    pub fn removeGroupFromChatsForCharacter(
        &self,
        groupName: &str,
        characterCardName: &str,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = NULL, updatedAt = ?3 WHERE \"group\" = ?1 AND characterCardName = ?2",
            sqliteParams![groupName, characterCardName, timestamp],
        )
    }

    pub fn removeGroupFromLockedChatsForCharacter(
        &self,
        groupName: &str,
        characterCardName: &str,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = NULL, updatedAt = ?3 WHERE \"group\" = ?1 AND characterCardName = ?2 AND locked = 1",
            sqliteParams![groupName, characterCardName, timestamp],
        )
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

    pub fn getChatsByCharacterCard(
        &self,
        characterCardName: &str,
    ) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChatsWithOne(
            "SELECT * FROM chats WHERE characterCardName = ?1 AND characterGroupId IS NULL ORDER BY pinned DESC, displayOrder ASC",
            characterCardName,
        )
    }

    pub fn getChatsByCharacterGroupId(
        &self,
        characterGroupId: &str,
    ) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChatsWithOne(
            "SELECT * FROM chats WHERE characterGroupId = ?1 ORDER BY pinned DESC, displayOrder ASC",
            characterGroupId,
        )
    }

    pub fn getChatsByCharacterCardOrNull(
        &self,
        characterCardName: &str,
    ) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChatsWithOne(
            "SELECT * FROM chats WHERE characterCardName = ?1 OR (characterCardName IS NULL AND characterGroupId IS NULL) ORDER BY pinned DESC, displayOrder ASC",
            characterCardName,
        )
    }

    pub fn clearCharacterCardBinding(
        &self,
        characterCardName: &str,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET characterCardName = NULL, updatedAt = ?2 WHERE characterCardName = ?1",
            sqliteParams![characterCardName, timestamp],
        )
    }

    pub fn deleteUnlockedChatsByCharacterCardName(
        &self,
        characterCardName: &str,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "DELETE FROM chats WHERE characterCardName = ?1 AND locked = 0",
            sqliteParams![characterCardName],
        )
    }

    pub fn deleteUnlockedUnboundChats(&self) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "DELETE FROM chats WHERE characterCardName IS NULL AND characterGroupId IS NULL AND locked = 0",
            sqliteParams![],
        )
    }

    pub fn renameCharacterCardBinding(
        &self,
        oldName: &str,
        newName: &str,
        timestamp: i64,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "UPDATE chats SET characterCardName = ?2, updatedAt = ?3 WHERE characterCardName = ?1",
            sqliteParams![oldName, newName, timestamp],
        )
    }

    pub fn renameCharacterGroupBinding(
        &self,
        sourceGroupId: &str,
        targetGroupId: &str,
        timestamp: i64,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "UPDATE chats SET characterCardName = NULL, characterGroupId = ?2, updatedAt = ?3 WHERE characterGroupId = ?1",
            sqliteParams![sourceGroupId, targetGroupId, timestamp],
        )
    }

    pub fn assignCharacterCardToUnbound(
        &self,
        newName: &str,
        timestamp: i64,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "UPDATE chats SET characterCardName = ?1, updatedAt = ?2 WHERE characterCardName IS NULL AND characterGroupId IS NULL",
            sqliteParams![newName, timestamp],
        )
    }

    pub fn assignCharacterGroupToUnbound(
        &self,
        targetGroupId: &str,
        timestamp: i64,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "UPDATE chats SET characterCardName = NULL, characterGroupId = ?1, updatedAt = ?2 WHERE characterGroupId IS NULL AND characterCardName IS NULL",
            sqliteParams![targetGroupId, timestamp],
        )
    }

    pub fn updateCharacterCardForChats(
        &self,
        chatIds: Vec<String>,
        newName: Option<String>,
        timestamp: i64,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_for_chat_ids(
            "UPDATE chats SET characterCardName = ?, characterGroupId = NULL, updatedAt = ? WHERE id IN",
            chatIds,
            sqliteParams![newName, timestamp],
        )
    }

    pub fn updateCharacterGroupForChats(
        &self,
        chatIds: Vec<String>,
        characterGroupId: Option<String>,
        timestamp: i64,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_for_chat_ids(
            "UPDATE chats SET characterCardName = NULL, characterGroupId = ?, updatedAt = ? WHERE id IN",
            chatIds,
            sqliteParams![characterGroupId, timestamp],
        )
    }

    pub fn clearCharacterGroupForChats(
        &self,
        chatIds: Vec<String>,
        timestamp: i64,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_for_chat_ids(
            "UPDATE chats SET characterGroupId = NULL, updatedAt = ? WHERE id IN",
            chatIds,
            sqliteParams![timestamp],
        )
    }

    pub fn clearCharacterGroupBinding(
        &self,
        sourceGroupId: &str,
        timestamp: i64,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "UPDATE chats SET characterGroupId = NULL, updatedAt = ?2 WHERE characterGroupId = ?1",
            sqliteParams![sourceGroupId, timestamp],
        )
    }

    pub fn updateGroupForChats(
        &self,
        chatIds: Vec<String>,
        groupName: Option<String>,
        timestamp: i64,
    ) -> Result<i32, SqliteStoreError> {
        self.execute_for_chat_ids(
            "UPDATE chats SET \"group\" = ?, updatedAt = ? WHERE id IN",
            chatIds,
            sqliteParams![groupName, timestamp],
        )
    }

    pub fn getCharacterCardChatStats(
        &self,
    ) -> Result<Vec<CharacterCardChatStats>, SqliteStoreError> {
        self.store
            .queryRows(
                r#"
                SELECT c.characterCardName AS characterCardName,
                    COUNT(c.id) AS chatCount,
                    IFNULL(SUM(mc.messageCount), 0) AS messageCount
                FROM chats c
                LEFT JOIN (
                    SELECT chatId, COUNT(*) AS messageCount
                    FROM messages
                    GROUP BY chatId
                ) mc ON c.id = mc.chatId
                WHERE c.characterGroupId IS NULL
                GROUP BY c.characterCardName
                "#,
                sqliteParams![],
            )?
            .into_iter()
            .map(|row| {
                Ok(CharacterCardChatStats {
                    characterCardName: row.get(0)?,
                    chatCount: row.get(1)?,
                    messageCount: row.get(2)?,
                })
            })
            .collect()
    }

    pub fn getCharacterGroupChatStats(
        &self,
    ) -> Result<Vec<CharacterGroupChatStats>, SqliteStoreError> {
        self.store
            .queryRows(
                r#"
                SELECT c.characterGroupId AS characterGroupId,
                    COUNT(c.id) AS chatCount,
                    IFNULL(SUM(mc.messageCount), 0) AS messageCount
                FROM chats c
                LEFT JOIN (
                    SELECT chatId, COUNT(*) AS messageCount
                    FROM messages
                    GROUP BY chatId
                ) mc ON c.id = mc.chatId
                WHERE c.characterCardName IS NULL
                GROUP BY c.characterGroupId
                "#,
                sqliteParams![],
            )?
            .into_iter()
            .map(|row| {
                Ok(CharacterGroupChatStats {
                    characterGroupId: row.get(0)?,
                    chatCount: row.get(1)?,
                    messageCount: row.get(2)?,
                })
            })
            .collect()
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

pub fn mapChatEntity(row: &SqliteRow) -> Result<ChatEntity, SqliteStoreError> {
    Ok(ChatEntity {
        id: row.get("id")?,
        title: row.get("title")?,
        createdAt: row.get("createdAt")?,
        updatedAt: row.get("updatedAt")?,
        inputTokens: row.get("inputTokens")?,
        outputTokens: row.get("outputTokens")?,
        currentWindowSize: row.get("currentWindowSize")?,
        group: row.get("group")?,
        displayOrder: row.get("displayOrder")?,
        workspace: row.get("workspace")?,
        parentChatId: row.get("parentChatId")?,
        characterCardName: row.get("characterCardName")?,
        characterGroupId: row.get("characterGroupId")?,
        locked: row.get("locked")?,
        pinned: row.get("pinned")?,
    })
}

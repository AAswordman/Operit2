use operit_store::SqliteStore::{SqliteStore, SqliteStoreError};
use rusqlite::{params, params_from_iter, Row};

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

    pub fn getAllChats(&self) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.getAllChatsDirectly()
    }

    pub fn getTotalChatCount(&self) -> Result<i32, SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.query_row("SELECT COUNT(*) FROM chats", [], |row| row.get(0))
        })
    }

    pub fn getAllChatsDirectly(&self) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChats("SELECT * FROM chats ORDER BY displayOrder ASC", [])
    }

    pub fn getChatById(&self, chatId: &str) -> Result<Option<ChatEntity>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare("SELECT * FROM chats WHERE id = ?1")?;
            let result = statement.query_row(params![chatId], mapChatEntity);
            match result {
                Ok(chat) => Ok(Some(chat)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(error) => Err(error),
            }
        })
    }

    pub fn insertChat(&self, chat: ChatEntity) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(
                r#"
                INSERT OR REPLACE INTO chats (
                    id, title, createdAt, updatedAt, inputTokens, outputTokens,
                    currentWindowSize, "group", displayOrder, workspace, workspaceEnv,
                    parentChatId, characterCardName, characterGroupId, locked
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                "#,
                params![
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
                    chat.workspaceEnv,
                    chat.parentChatId,
                    chat.characterCardName,
                    chat.characterGroupId,
                    chat.locked,
                ],
            )?;
            Ok(())
        })
    }

    pub fn deleteChat(&self, chatId: &str) -> Result<(), SqliteStoreError> {
        self.execute("DELETE FROM chats WHERE id = ?1", params![chatId])
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
            params![chatId, timestamp, title, inputTokens, outputTokens, currentWindowSize],
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
            params![chatId, title, timestamp],
        )
    }

    pub fn updateChatWorkspace(
        &self,
        chatId: &str,
        workspace: Option<String>,
        workspaceEnv: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET workspace = ?2, workspaceEnv = ?3, updatedAt = ?4 WHERE id = ?1",
            params![chatId, workspace, workspaceEnv, timestamp],
        )
    }

    pub fn updateChatTitleAndWorkspace(
        &self,
        chatId: &str,
        title: String,
        workspace: Option<String>,
        workspaceEnv: Option<String>,
        timestamp: i64,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET title = ?2, workspace = ?3, workspaceEnv = ?4, updatedAt = ?5 WHERE id = ?1",
            params![chatId, title, workspace, workspaceEnv, timestamp],
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
            params![chatId, group, timestamp],
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
            params![chatId, characterCardName, timestamp],
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
            params![chatId, characterGroupId, timestamp],
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
            params![chatId, characterCardName, characterGroupId, timestamp],
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
            params![chatId, locked, timestamp],
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
            params![chatId, displayOrder, group, timestamp],
        )
    }

    pub fn updateChats(&self, chats: Vec<ChatEntity>) -> Result<(), SqliteStoreError> {
        for chat in chats {
            self.insertChat(chat)?;
        }
        Ok(())
    }

    pub fn updateGroupName(&self, oldName: &str, newName: &str) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = ?2 WHERE \"group\" = ?1",
            params![oldName, newName],
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
            params![oldName, newName, characterCardName],
        )
    }

    pub fn deleteChatsInGroup(&self, groupName: &str) -> Result<(), SqliteStoreError> {
        self.execute(
            "DELETE FROM chats WHERE \"group\" = ?1 AND locked = 0",
            params![groupName],
        )
    }

    pub fn deleteChatsInGroupForCharacter(
        &self,
        groupName: &str,
        characterCardName: &str,
    ) -> Result<(), SqliteStoreError> {
        self.execute(
            "DELETE FROM chats WHERE \"group\" = ?1 AND characterCardName = ?2 AND locked = 0",
            params![groupName, characterCardName],
        )
    }

    pub fn removeGroupFromChats(&self, groupName: &str, timestamp: i64) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = NULL, updatedAt = ?2 WHERE \"group\" = ?1",
            params![groupName, timestamp],
        )
    }

    pub fn removeGroupFromLockedChats(&self, groupName: &str, timestamp: i64) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET \"group\" = NULL, updatedAt = ?2 WHERE \"group\" = ?1 AND locked = 1",
            params![groupName, timestamp],
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
            params![groupName, characterCardName, timestamp],
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
            params![groupName, characterCardName, timestamp],
        )
    }

    pub fn getBranchesByParentId(&self, parentChatId: &str) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChatsWithOne(
            "SELECT * FROM chats WHERE parentChatId = ?1 ORDER BY displayOrder ASC",
            parentChatId,
        )
    }

    pub fn getBranchesByParentIdFlow(&self, parentChatId: &str) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.getBranchesByParentId(parentChatId)
    }

    pub fn getMainChats(&self) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChats("SELECT * FROM chats WHERE parentChatId IS NULL ORDER BY displayOrder ASC", [])
    }

    pub fn getMainChatsFlow(&self) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.getMainChats()
    }

    pub fn getChatsByCharacterCard(&self, characterCardName: &str) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChatsWithOne(
            "SELECT * FROM chats WHERE characterCardName = ?1 AND characterGroupId IS NULL ORDER BY displayOrder ASC",
            characterCardName,
        )
    }

    pub fn getChatsByCharacterGroupId(&self, characterGroupId: &str) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChatsWithOne(
            "SELECT * FROM chats WHERE characterGroupId = ?1 ORDER BY displayOrder ASC",
            characterGroupId,
        )
    }

    pub fn getChatsByCharacterCardOrNull(&self, characterCardName: &str) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChatsWithOne(
            "SELECT * FROM chats WHERE characterCardName = ?1 OR (characterCardName IS NULL AND characterGroupId IS NULL) ORDER BY displayOrder ASC",
            characterCardName,
        )
    }

    pub fn clearCharacterCardBinding(&self, characterCardName: &str, timestamp: i64) -> Result<(), SqliteStoreError> {
        self.execute(
            "UPDATE chats SET characterCardName = NULL, updatedAt = ?2 WHERE characterCardName = ?1",
            params![characterCardName, timestamp],
        )
    }

    pub fn deleteUnlockedChatsByCharacterCardName(&self, characterCardName: &str) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "DELETE FROM chats WHERE characterCardName = ?1 AND locked = 0",
            params![characterCardName],
        )
    }

    pub fn deleteUnlockedUnboundChats(&self) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "DELETE FROM chats WHERE characterCardName IS NULL AND characterGroupId IS NULL AND locked = 0",
            [],
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
            params![oldName, newName, timestamp],
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
            params![sourceGroupId, targetGroupId, timestamp],
        )
    }

    pub fn assignCharacterCardToUnbound(&self, newName: &str, timestamp: i64) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "UPDATE chats SET characterCardName = ?1, updatedAt = ?2 WHERE characterCardName IS NULL AND characterGroupId IS NULL",
            params![newName, timestamp],
        )
    }

    pub fn assignCharacterGroupToUnbound(&self, targetGroupId: &str, timestamp: i64) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "UPDATE chats SET characterCardName = NULL, characterGroupId = ?1, updatedAt = ?2 WHERE characterGroupId IS NULL AND characterCardName IS NULL",
            params![targetGroupId, timestamp],
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
            vec![&newName as &dyn rusqlite::ToSql, &timestamp],
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
            vec![&characterGroupId as &dyn rusqlite::ToSql, &timestamp],
        )
    }

    pub fn clearCharacterGroupForChats(&self, chatIds: Vec<String>, timestamp: i64) -> Result<i32, SqliteStoreError> {
        self.execute_for_chat_ids(
            "UPDATE chats SET characterGroupId = NULL, updatedAt = ? WHERE id IN",
            chatIds,
            vec![&timestamp as &dyn rusqlite::ToSql],
        )
    }

    pub fn clearCharacterGroupBinding(&self, sourceGroupId: &str, timestamp: i64) -> Result<i32, SqliteStoreError> {
        self.execute_count(
            "UPDATE chats SET characterGroupId = NULL, updatedAt = ?2 WHERE characterGroupId = ?1",
            params![sourceGroupId, timestamp],
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
            vec![&groupName as &dyn rusqlite::ToSql, &timestamp],
        )
    }

    pub fn getCharacterCardChatStats(&self) -> Result<Vec<CharacterCardChatStats>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
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
            )?;
            let rows = statement.query_map([], |row| {
                Ok(CharacterCardChatStats {
                    characterCardName: row.get(0)?,
                    chatCount: row.get(1)?,
                    messageCount: row.get(2)?,
                })
            })?;
            rows.collect()
        })
    }

    pub fn getCharacterGroupChatStats(&self) -> Result<Vec<CharacterGroupChatStats>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(
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
            )?;
            let rows = statement.query_map([], |row| {
                Ok(CharacterGroupChatStats {
                    characterGroupId: row.get(0)?,
                    chatCount: row.get(1)?,
                    messageCount: row.get(2)?,
                })
            })?;
            rows.collect()
        })
    }

    fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<(), SqliteStoreError> {
        self.store.withConnection(|connection| {
            connection.execute(sql, params)?;
            Ok(())
        })
    }

    fn execute_count<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<i32, SqliteStoreError> {
        self.store.withConnection(|connection| Ok(connection.execute(sql, params)? as i32))
    }

    fn selectChats<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.store.withConnection(|connection| {
            let mut statement = connection.prepare(sql)?;
            let rows = statement.query_map(params, mapChatEntity)?;
            rows.collect()
        })
    }

    fn selectChatsWithOne(&self, sql: &str, value: &str) -> Result<Vec<ChatEntity>, SqliteStoreError> {
        self.selectChats(sql, params![value])
    }

    fn execute_for_chat_ids(
        &self,
        sqlPrefix: &str,
        chatIds: Vec<String>,
        mut leadingParams: Vec<&dyn rusqlite::ToSql>,
    ) -> Result<i32, SqliteStoreError> {
        let placeholders = chatIds.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("{sqlPrefix} ({placeholders})");
        self.store.withConnection(|connection| {
            for chatId in &chatIds {
                leadingParams.push(chatId);
            }
            Ok(connection.execute(&sql, params_from_iter(leadingParams))? as i32)
        })
    }
}

pub fn mapChatEntity(row: &Row<'_>) -> Result<ChatEntity, rusqlite::Error> {
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
        workspaceEnv: row.get("workspaceEnv")?,
        parentChatId: row.get("parentChatId")?,
        characterCardName: row.get("characterCardName")?,
        characterGroupId: row.get("characterGroupId")?,
        locked: row.get("locked")?,
    })
}

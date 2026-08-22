use anyhow::{Context, Result};
use rusqlite::params;
use uuid::Uuid;

use crate::meeting::models::MeetingFolderView;

use super::paths::wall_ms;
use super::MeetingStore;

impl MeetingStore {
    pub fn list_folders(&self) -> Result<Vec<MeetingFolderView>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, parent_id, sort_order, created_at_ms
             FROM meeting_folder ORDER BY sort_order ASC, name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MeetingFolderView {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                sort_order: row.get(3)?,
                created_at_ms: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().context("list folders")
    }

    pub fn create_folder(&self, name: &str, parent_id: Option<&str>) -> Result<MeetingFolderView> {
        let id = Uuid::new_v4().to_string();
        let now = wall_ms();
        let conn = self.conn();
        let sort_order: i32 = conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM meeting_folder",
            [],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO meeting_folder (id, name, parent_id, sort_order, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, parent_id, sort_order, now],
        )?;
        Ok(MeetingFolderView {
            id,
            name: name.to_string(),
            parent_id: parent_id.map(str::to_string),
            sort_order,
            created_at_ms: now,
        })
    }

    pub fn reorder_folders(&self, folder_ids: &[String]) -> Result<Vec<MeetingFolderView>> {
        {
            let conn = self.conn();
            let total: i64 =
                conn.query_row("SELECT COUNT(*) FROM meeting_folder", [], |row| row.get(0))?;
            if folder_ids.len() as i64 != total {
                anyhow::bail!("folder reorder list must include every folder");
            }
            let tx = conn.unchecked_transaction()?;
            for (index, id) in folder_ids.iter().enumerate() {
                let updated = tx.execute(
                    "UPDATE meeting_folder SET sort_order = ?1 WHERE id = ?2",
                    params![index as i32, id],
                )?;
                if updated == 0 {
                    anyhow::bail!("folder not found: {id}");
                }
            }
            tx.commit()?;
        }
        self.list_folders()
    }

    pub fn rename_folder(&self, id: &str, name: &str) -> Result<MeetingFolderView> {
        {
            let conn = self.conn();
            let updated = conn.execute(
                "UPDATE meeting_folder SET name = ?1 WHERE id = ?2",
                params![name, id],
            )?;
            if updated == 0 {
                anyhow::bail!("folder not found");
            }
        }
        self.get_folder(id)
    }

    pub fn get_folder(&self, id: &str) -> Result<MeetingFolderView> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, name, parent_id, sort_order, created_at_ms FROM meeting_folder WHERE id = ?1",
            params![id],
            |row| {
                Ok(MeetingFolderView {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    parent_id: row.get(2)?,
                    sort_order: row.get(3)?,
                    created_at_ms: row.get(4)?,
                })
            },
        )
        .context("folder not found")
    }

    pub fn delete_folder(&self, id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE meeting_record SET folder_id = NULL WHERE folder_id = ?1",
            params![id],
        )?;
        let deleted = conn.execute("DELETE FROM meeting_folder WHERE id = ?1", params![id])?;
        if deleted == 0 {
            anyhow::bail!("folder not found");
        }
        Ok(())
    }
}

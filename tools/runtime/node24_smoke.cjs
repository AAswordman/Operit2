'use strict';

const assert = require('node:assert/strict');
const { rmSync } = require('node:fs');
const { DatabaseSync } = require('node:sqlite');

assert.equal(process.versions.node.split('.')[0], '24');
const databasePath = `/tmp/operit-node24-smoke-${process.pid}.sqlite`;
try {
  const database = new DatabaseSync(databasePath);
  database.exec('CREATE TABLE questions (id INTEGER PRIMARY KEY, text TEXT NOT NULL)');
  database.prepare('INSERT INTO questions (text) VALUES (?)').run('Ferry Node 24');
  database.close();
  const reopened = new DatabaseSync(databasePath);
  assert.equal(reopened.prepare('SELECT text FROM questions WHERE id = ?').get(1).text, 'Ferry Node 24');
  reopened.close();
  console.log(`Node ${process.version} (${process.arch}): node:sqlite create/insert/reopen/query passed`);
} finally {
  rmSync(databasePath, { force: true });
}

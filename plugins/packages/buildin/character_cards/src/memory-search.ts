import type { Storage } from "../../../../types/storage";
import type { Memory, MemorySearchConfig, MemorySearchOptions, MemorySearchDebug, MemorySettings } from "./model";
import type { CharacterMemoryDatabase } from "./storage/memory";
import type { SourceEmbedding } from "./memory-jobs/embeddings";
import { embedTexts } from "./memory-jobs/embeddings";

export const MEMORY_SEARCH_THRESHOLD = 0.025;

/** Expands Unicode query words, Han bigrams and unigrams under a fixed lexical budget. */
function lexicalTerms(query: string): string[] {
  const terms = new Set<string>();
  for (const match of query.toLowerCase().matchAll(/[\p{L}\p{N}_*]+/gu)) {
    const word = match[0]; terms.add(word);
    for (const han of word.matchAll(/\p{Script=Han}+/gu)) {
      const chars = [...han[0]];
      for (let index = 0; index < chars.length - 1; index += 1) terms.add(chars[index] + chars[index + 1]);
      for (const char of chars) terms.add(char);
    }
  }
  return [...terms].slice(0, 32);
}

/** Builds parameters for owner, folder, time and exclusion filters before ranking. */
function parameters(options: MemorySearchOptions): Storage.SqlValue[] {
  return [options.ownerKey + "/", options.ownerKey + "0", options.folderPath, options.createdAtStartMs, options.createdAtEndMs,
    options.query, JSON.stringify(lexicalTerms(options.query))];
}

/** Scores persisted source vectors and lexical matches entirely inside one SQLite snapshot.
 * Keep CTEs free of materialization hints: the host SQL validator uses SQLiteDialect. */
const rankedQuery = `
WITH query_vector AS (SELECT CAST(key AS INTEGER) AS dimension,value FROM json_each(?9)), scoped AS (
  SELECT key,value,json_extract(value,'$.record.id') AS id,
    json_extract(value,'$.record.importance') AS importance,
    json_extract(value,'$.record.updatedAt') AS updatedAt
  FROM plugin_objects
  WHERE collection='memories' AND key>=?1 AND key<?2
    AND json_extract(value,'$.record.title')<>'.folder_placeholder'
    AND (?3 IS NULL OR json_extract(value,'$.record.folderPath')=?3 OR
      substr(json_extract(value,'$.record.folderPath'),1,length(?3)+1)=?3||'/')
    AND (?4 IS NULL OR json_extract(value,'$.record.createdAt')>=?4)
    AND (?5 IS NULL OR json_extract(value,'$.record.createdAt')<=?5)
), units AS (
  SELECT id,value FROM scoped
  UNION ALL
  SELECT m.id,c.value FROM plugin_objects c JOIN scoped m
    ON json_extract(c.value,'$.record.memoryUuid')=json_extract(m.value,'$.record.uuid')
  WHERE c.collection='chunks' AND c.key>=?1 AND c.key<?2
), lexical AS (
  SELECT m.*,
    (SELECT count(*) FROM json_each(?7) t WHERE
      lower(json_extract(m.value,'$.record.title')) GLOB '*'||t.value||'*'
      OR EXISTS (SELECT 1 FROM units u WHERE u.id=m.id AND
        lower(json_extract(u.value,'$.record.content')) GLOB '*'||t.value||'*')) AS keywords,
    (SELECT count(*) FROM json_each(?7) t WHERE EXISTS (
      SELECT 1 FROM json_each(m.value,'$.record.tags') tag
      WHERE lower(json_extract(tag.value,'$.name')) GLOB '*'||t.value||'*')) AS tags
  FROM scoped m
), keyword_ranks AS (
  SELECT id,?10*importance*(1+0.6*keywords/max(1,json_array_length(?7)))/(60.0+row_number() OVER (ORDER BY keywords DESC,importance DESC,updatedAt DESC,id)) AS score
  FROM lexical WHERE keywords>0
), tag_ranks AS (
  SELECT id,?11*importance*(1+0.6*tags/max(1,json_array_length(?7)))/(60.0+row_number() OVER (ORDER BY tags DESC,importance DESC,updatedAt DESC,id)) AS score
  FROM lexical WHERE tags>0
), similarities AS (
  SELECT u.id,max((SELECT sum(q.value*v.value) FROM json_each(u.value,'$.embedding.vector') v
    JOIN query_vector q ON q.dimension=v.key)) AS similarity
  FROM units u WHERE ?12>0 AND json_extract(u.value,'$.embedding.space')=?8
    AND json_array_length(u.value,'$.embedding.vector')=json_array_length(?9)
  GROUP BY u.id
), semantic AS (
  SELECT id,?12*max(-1.0,min(1.0,similarity)) AS score FROM similarities
), base AS (
  SELECT m.*,coalesce(k.score,0)+CASE WHEN instr(lower(?6),lower(json_extract(m.value,'$.record.title')))>0 THEN ?10*m.importance/61.0 ELSE 0 END AS keywordScore,coalesce(t.score,0) AS tagScore,
    coalesce(s.score,0) AS vectorScore,
    coalesce(k.score,0)+coalesce(t.score,0)+coalesce(s.score,0)+CASE WHEN instr(lower(?6),lower(json_extract(m.value,'$.record.title')))>0 THEN ?10*m.importance/61.0 ELSE 0 END AS baseScore
  FROM scoped m LEFT JOIN keyword_ranks k USING(id) LEFT JOIN tag_ranks t USING(id) LEFT JOIN semantic s USING(id)
), seeds AS (
  SELECT id,baseScore FROM base WHERE baseScore>0 ORDER BY baseScore DESC,id LIMIT 10
), edges AS (
  SELECT target.id,sum((seed.baseScore*json_extract(l.value,'$.record.weight')+0.03)*?13) AS score
  FROM plugin_objects l JOIN seeds seed ON
    json_extract(l.value,'$.record.sourceMemoryId')=seed.id OR json_extract(l.value,'$.record.targetMemoryId')=seed.id
  JOIN scoped target ON target.id=CASE WHEN json_extract(l.value,'$.record.sourceMemoryId')=seed.id
    THEN json_extract(l.value,'$.record.targetMemoryId') ELSE json_extract(l.value,'$.record.sourceMemoryId') END
  WHERE l.collection='links' AND l.key>=?1 AND l.key<?2 AND ?13>0 GROUP BY target.id
), scored AS (
  SELECT b.*,coalesce(e.score,0) AS edgeScore,baseScore+coalesce(e.score,0) AS score FROM base b LEFT JOIN edges e USING(id)
), page AS (
  SELECT * FROM scored WHERE (?17=1 OR (score>0 AND score>=?14)) AND id NOT IN (SELECT value FROM json_each(?16)) ORDER BY score DESC,id LIMIT ?15
), stats AS (
  SELECT (SELECT count(*) FROM scoped) AS total,
    (SELECT count(*) FROM scored WHERE score>0 AND score>=?14 AND id IN (SELECT value FROM json_each(?16))) AS excluded,
    (SELECT count(*) FROM units WHERE ?12>0 AND (
      json_extract(value,'$.embedding.text') IS NOT CASE WHEN json_extract(value,'$.record.isDocumentNode')=1 THEN json_extract(value,'$.record.title') ELSE json_extract(value,'$.record.content') END OR
      json_extract(value,'$.embedding.space') IS NOT ?8 OR
      json_array_length(value,'$.embedding.vector') IS NOT json_array_length(?9))) AS invalid
)
SELECT json_extract(page.value,'$.record') AS record,page.keywordScore,page.tagScore,page.vectorScore,page.edgeScore,page.score,stats.total,stats.invalid,stats.excluded
FROM stats LEFT JOIN page ON 1 ORDER BY page.score DESC,page.id`;

/** Prepares one ephemeral vector shared by all parts of one retrieval operation. */
export async function prepareMemoryQuery(settings: MemorySettings, query: string, config: MemorySearchConfig): Promise<SourceEmbedding | null> {
  return query.trim() !== "*" && settings.cloudEmbeddingEnabled && config.vectorWeight > 0 ? (await embedTexts([query], settings))[0] : null;
}

/** Queries bounded memory hits and retains the ephemeral query input only for this operation. */
export async function searchMemoryDatabase(database: CharacterMemoryDatabase, settings: MemorySettings, options: MemorySearchOptions, config: MemorySearchConfig, debug = false): Promise<{ result: MemorySearchDebug; queryEmbedding: SourceEmbedding | null }> {
  const limit = options.limit ?? 100;
  if (!Number.isInteger(limit) || limit < 1 || limit > 1000) throw new Error("Memory search limit must be between 1 and 1000");
  const params = parameters(options);
  if (options.query.trim() === "*") {
    const rows = await database.query(`WITH scoped AS (
      SELECT value,json_extract(value,'$.record.id') AS id FROM plugin_objects WHERE collection='memories' AND key>=?1 AND key<?2
      AND (?3 IS NULL OR json_extract(value,'$.record.folderPath')=?3 OR substr(json_extract(value,'$.record.folderPath'),1,length(?3)+1)=?3||'/')
      AND (?4 IS NULL OR json_extract(value,'$.record.createdAt')>=?4) AND (?5 IS NULL OR json_extract(value,'$.record.createdAt')<=?5)
      AND json_extract(value,'$.record.title')<>'.folder_placeholder'
    ), page AS (SELECT * FROM scoped WHERE id NOT IN (SELECT value FROM json_each(?6)) ORDER BY id LIMIT ?7), stats AS (
      SELECT count(*) AS total,sum(CASE WHEN id IN (SELECT value FROM json_each(?6)) THEN 1 ELSE 0 END) AS excluded FROM scoped
    ) SELECT json_extract(page.value,'$.record') AS record,stats.total,stats.excluded FROM stats LEFT JOIN page ON 1 ORDER BY page.id`,
      [...params.slice(0, 5), JSON.stringify(options.excludeIds ?? []), limit]);
    return { queryEmbedding: null, result: { excludedBySnapshotCount: Number(rows[0].excluded), query: options.query, config, relevanceThreshold: options.relevanceThreshold, memoriesInScopeCount: Number(rows[0].total),
      candidates: rows.filter(/** Omits an empty listing page. */ row => row.record !== null).map(/** Returns complete explicitly listed records. */ row => ({ memory: JSON.parse(String(row.record)) as Memory, score: 1, keywordScore: 0, tagScore: 0, vectorScore: 0, edgeScore: 0, passedThreshold: true })) } };
  }
  if (options.query.trim() === "") throw new Error("Memory search query must not be blank");
  const vectorWeight = settings.cloudEmbeddingEnabled ? config.vectorWeight : 0;
  if (lexicalTerms(options.query).length === 0 && vectorWeight === 0) throw new Error("Memory search query has no searchable tokens");
  if (config.keywordWeight + config.tagWeight + vectorWeight + config.edgeWeight === 0) throw new Error("Memory search has no enabled scoring weights");
  const queryEmbedding = await prepareMemoryQuery(settings, options.query, config);
  const space = queryEmbedding === null ? "" : queryEmbedding.space, vector = queryEmbedding === null ? [] : queryEmbedding.vector;
  const keywordBoost = config.scoreMode === "KEYWORD_FIRST" ? 1.3 : config.scoreMode === "SEMANTIC_FIRST" ? 0.8 : 1;
  const vectorBoost = config.scoreMode === "SEMANTIC_FIRST" ? 1.3 : config.scoreMode === "KEYWORD_FIRST" ? 0.8 : 1;
  params.push(space, JSON.stringify(vector), config.keywordWeight * keywordBoost, config.tagWeight * keywordBoost, vectorWeight * vectorBoost,
    config.edgeWeight * (config.scoreMode === "KEYWORD_FIRST" ? 0.9 : config.scoreMode === "SEMANTIC_FIRST" ? 1.1 : 1), options.relevanceThreshold, limit, JSON.stringify(options.excludeIds ?? []), debug ? 1 : 0);
  const rows = await database.query(rankedQuery, params);
  if (Number(rows[0].invalid) !== 0) throw new Error("Memory vector index is incomplete or its provider dimensions changed; rebuild memory embeddings");
  return { queryEmbedding, result: { excludedBySnapshotCount: Number(rows[0].excluded), query: options.query, config, relevanceThreshold: options.relevanceThreshold, memoriesInScopeCount: Number(rows[0].total),
    candidates: rows.filter(/** Omits the aggregate-only row for an empty result page. */ row => row.record !== null).map(
      /** Decodes bounded hits after all database filtering and ranking. */ row => ({ memory: JSON.parse(String(row.record)) as Memory,
        score: Number(row.score), keywordScore: Number(row.keywordScore), tagScore: Number(row.tagScore), vectorScore: Number(row.vectorScore), edgeScore: Number(row.edgeScore),
        passedThreshold: Number(row.score) > 0 && Number(row.score) >= options.relevanceThreshold })) } };
}

package app.operit

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.DocumentsContract.Document
import java.io.FileNotFoundException
import java.io.InputStream
import java.io.OutputStream

/** Provider boundary, injectable for filesystem contract tests with opaque IDs. */
interface DocumentTreeAccess {
    fun hasReadGrant(tree: String): Boolean
    fun hasWriteGrant(tree: String): Boolean
    fun rootId(tree: String): String
    fun children(tree: String, parentId: String): List<DocumentMetadata>
    fun describe(tree: String, documentId: String): DocumentMetadata
    fun create(tree: String, parentId: String, name: String, directory: Boolean): String
    fun delete(tree: String, documentId: String): Boolean
    fun input(tree: String, documentId: String): InputStream
    fun output(tree: String, documentId: String, append: Boolean): OutputStream
}

data class DocumentMetadata(
    val id: String,
    val name: String,
    val directory: Boolean,
    val size: Long,
    val modified: String,
    val writable: Boolean,
    val deletable: Boolean = true,
)

/** Each URI is constructed using the original authorized tree, never a raw file path. */
class ContentResolverDocumentTreeAccess(context: Context) : DocumentTreeAccess {
    private val resolverContext = context.applicationContext
    private val resolver = resolverContext.contentResolver
    private fun tree(value: String): Uri {
        val uri = Uri.parse(value)
        require(uri.scheme == "content" && !uri.authority.isNullOrEmpty() && uri.query == null && uri.fragment == null &&
            (DocumentsContract.isTreeUri(uri) || DocumentsContract.isDocumentUri(resolverContext, uri))) {
            "Expected an authorized content document or tree URI"
        }
        return uri
    }
    private fun document(tree: String, id: String): Uri {
        val uri = tree(tree)
        if (DocumentsContract.isTreeUri(uri)) return DocumentsContract.buildDocumentUriUsingTree(uri, id)
        require(id == DocumentsContract.getDocumentId(uri)) { "A selected file does not authorize sibling documents" }
        return uri
    }

    override fun hasReadGrant(tree: String): Boolean {
        val uri = tree(tree)
        return resolver.persistedUriPermissions.any { it.uri == uri && it.isReadPermission }
    }
    override fun hasWriteGrant(tree: String): Boolean {
        val uri = tree(tree)
        return resolver.persistedUriPermissions.any { it.uri == uri && it.isWritePermission }
    }
    override fun rootId(tree: String): String {
        val uri = tree(tree)
        return if (DocumentsContract.isTreeUri(uri)) DocumentsContract.getTreeDocumentId(uri) else DocumentsContract.getDocumentId(uri)
    }

    private val columns = arrayOf(Document.COLUMN_DOCUMENT_ID, Document.COLUMN_DISPLAY_NAME,
        Document.COLUMN_MIME_TYPE, Document.COLUMN_SIZE, Document.COLUMN_LAST_MODIFIED, Document.COLUMN_FLAGS)
    private fun read(cursor: android.database.Cursor): DocumentMetadata {
        val flags = cursor.getLong(5)
        return DocumentMetadata(cursor.getString(0), cursor.getString(1), cursor.getString(2) == Document.MIME_TYPE_DIR,
            if (cursor.isNull(3)) 0 else cursor.getLong(3),
            if (cursor.isNull(4)) "" else cursor.getLong(4).toString(),
            flags and (Document.FLAG_SUPPORTS_WRITE or Document.FLAG_DIR_SUPPORTS_CREATE).toLong() != 0L,
            flags and Document.FLAG_SUPPORTS_DELETE.toLong() != 0L)
    }
    override fun children(tree: String, parentId: String): List<DocumentMetadata> {
        val uri = DocumentsContract.buildChildDocumentsUriUsingTree(tree(tree), parentId)
        val result = ArrayList<DocumentMetadata>()
        resolver.query(uri, columns, null, null, null)?.use { cursor ->
            while (cursor.moveToNext()) result.add(read(cursor))
        } ?: throw IllegalStateException("Cannot query document children")
        return result
    }
    override fun describe(tree: String, documentId: String): DocumentMetadata {
        resolver.query(document(tree, documentId), columns, null, null, null)?.use { cursor ->
            if (!cursor.moveToFirst()) throw FileNotFoundException("Document does not exist")
            return read(cursor)
        } ?: throw IllegalStateException("Cannot query document metadata")
    }
    override fun create(tree: String, parentId: String, name: String, directory: Boolean): String {
        val created = DocumentsContract.createDocument(resolver, document(tree, parentId),
            if (directory) Document.MIME_TYPE_DIR else "application/octet-stream", name)
            ?: throw IllegalStateException("Provider could not create document")
        return DocumentsContract.getDocumentId(created)
    }
    override fun delete(tree: String, documentId: String): Boolean =
        DocumentsContract.deleteDocument(resolver, document(tree, documentId))
    override fun input(tree: String, documentId: String): InputStream =
        resolver.openInputStream(document(tree, documentId)) ?: throw IllegalStateException("Cannot read document")
    override fun output(tree: String, documentId: String, append: Boolean): OutputStream =
        resolver.openOutputStream(document(tree, documentId), if (append) "wa" else "wt")
            ?: throw IllegalStateException("Cannot write document")
}

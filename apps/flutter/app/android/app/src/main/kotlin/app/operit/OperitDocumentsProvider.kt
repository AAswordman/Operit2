package app.operit

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.database.Cursor
import android.database.MatrixCursor
import android.os.CancellationSignal
import android.os.Handler
import android.os.Looper
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import android.provider.DocumentsContract.Document
import android.provider.DocumentsContract.Root
import android.provider.DocumentsProvider
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import android.webkit.MimeTypeMap
import java.io.File
import java.io.FileNotFoundException
import java.util.Locale
import org.json.JSONObject

/** Implements local SAF operations without creating an Android Core runtime or Flutter engine. */
abstract class OperitDocumentsProvider : DocumentsProvider() {
    /** Returns only the directories owned by this provider's storage contract. */
    internal abstract fun roots(): List<DocumentFileStore>

    /** Registers the provider without reading bootstrap data or loading native libraries. */
    override fun onCreate(): Boolean = true

    /** Publishes roots independently of application startup and the maintenance storage lease. */
    override fun queryRoots(projection: Array<out String>?): Cursor {
        val cursor = MatrixCursor(projection ?: ROOT_COLUMNS)
        roots().forEach { root ->
            addRow(cursor, mapOf(
                Root.COLUMN_ROOT_ID to root.id,
                Root.COLUMN_DOCUMENT_ID to root.documentId,
                Root.COLUMN_TITLE to root.title,
                Root.COLUMN_SUMMARY to root.directory.path,
                Root.COLUMN_MIME_TYPES to "*/*",
                Root.COLUMN_ICON to R.mipmap.ic_launcher,
                Root.COLUMN_AVAILABLE_BYTES to root.directory.usableSpace,
                Root.COLUMN_FLAGS to (Root.FLAG_LOCAL_ONLY or Root.FLAG_SUPPORTS_IS_CHILD or
                    if (root.directory.canWrite()) Root.FLAG_SUPPORTS_CREATE else 0),
            ))
        }
        return cursor
    }

    /** Describes one existing document using the caller's requested projection. */
    override fun queryDocument(documentId: String, projection: Array<out String>?): Cursor {
        val root = root(documentId)
        return MatrixCursor(projection ?: DOCUMENT_COLUMNS).apply {
            include(this, root, root.file(documentId))
            setNotificationUri(providerContext().contentResolver, DocumentsContract.buildDocumentUri(authority(), documentId))
        }
    }

    /** Lists a directory and subscribes the cursor to subsequent file changes. */
    override fun queryChildDocuments(parentDocumentId: String, projection: Array<out String>?, sortOrder: String?): Cursor {
        val root = root(parentDocumentId)
        return MatrixCursor(projection ?: DOCUMENT_COLUMNS).apply {
            root.children(parentDocumentId).forEach { include(this, root, it) }
            setNotificationUri(providerContext().contentResolver,
                DocumentsContract.buildChildDocumentsUri(authority(), parentDocumentId))
        }
    }

    /** Returns the MIME type through the same validated document resolution as file access. */
    override fun getDocumentType(documentId: String): String = mimeType(root(documentId).file(documentId))

    /** Checks ancestry inside a single published root without accepting path prefixes as children. */
    override fun isChildDocument(parentDocumentId: String, documentId: String): Boolean {
        val parentRoot = root(parentDocumentId)
        val childRoot = root(documentId)
        if (parentRoot.id != childRoot.id) return false
        val parent = parentRoot.file(parentDocumentId)
        val child = childRoot.file(documentId)
        return parent.isDirectory && child != parent &&
            child.path.startsWith(parent.path.trimEnd(File.separatorChar) + File.separator)
    }

    /** Opens the original bytes and refreshes SAF clients when a writable descriptor closes. */
    override fun openDocument(documentId: String, mode: String, signal: CancellationSignal?): ParcelFileDescriptor {
        signal?.throwIfCanceled()
        val root = root(documentId)
        val file = root.file(documentId)
        if (!file.isFile) throw FileNotFoundException("Document is not a regular file")
        val access = ParcelFileDescriptor.parseMode(mode)
        if (access and ParcelFileDescriptor.MODE_WRITE_ONLY == 0) return ParcelFileDescriptor.open(file, access)
        return ParcelFileDescriptor.open(file, access, Handler(Looper.getMainLooper())) {
            notifyChange(root, documentId, file.parentFile)
        }
    }

    /** Creates a file or directory and refreshes its parent listing. */
    override fun createDocument(parentDocumentId: String, mimeType: String, displayName: String): String {
        val root = root(parentDocumentId)
        val created = root.create(parentDocumentId, displayName, mimeType == Document.MIME_TYPE_DIR)
        notifyChange(root, created, root.file(parentDocumentId))
        return created
    }

    /** Renames a document without overwriting another file or changing a published root. */
    override fun renameDocument(documentId: String, displayName: String): String {
        val root = root(documentId)
        val parent = root.file(documentId).parentFile
        val renamed = root.rename(documentId, displayName)
        notifyChange(root, documentId, parent)
        notifyChange(root, renamed, parent)
        return renamed
    }

    /** Deletes a non-root document and revokes its individual URI grants. */
    override fun deleteDocument(documentId: String) {
        val root = root(documentId)
        val parent = root.file(documentId).parentFile
        root.delete(documentId)
        revokeDocumentPermission(documentId)
        notifyChange(root, documentId, parent)
    }

    /** Selects the exact root embedded in the document ID, rejecting obsolete storage grants. */
    private fun root(documentId: String): DocumentFileStore = roots().firstOrNull {
        documentId.startsWith("${it.id}:")
    } ?: throw FileNotFoundException("Unknown or no longer published document root")

    /** Supplies metadata and advertises only supported operations with actual filesystem access. */
    private fun include(cursor: MatrixCursor, root: DocumentFileStore, file: File) {
        var flags = 0
        if (file.isDirectory && file.canWrite()) flags = flags or Document.FLAG_DIR_SUPPORTS_CREATE
        if (file.isFile && file.canWrite()) flags = flags or Document.FLAG_SUPPORTS_WRITE
        if (file != root.directory && file.parentFile?.canWrite() == true) {
            flags = flags or Document.FLAG_SUPPORTS_DELETE or Document.FLAG_SUPPORTS_RENAME
        }
        addRow(cursor, mapOf(
            Document.COLUMN_DOCUMENT_ID to root.documentId(file),
            Document.COLUMN_DISPLAY_NAME to if (file == root.directory) root.title else file.name,
            Document.COLUMN_MIME_TYPE to mimeType(file),
            Document.COLUMN_FLAGS to flags,
            Document.COLUMN_SIZE to if (file.isFile) file.length() else null,
            Document.COLUMN_LAST_MODIFIED to file.lastModified(),
        ))
    }

    /** Projects known metadata columns and returns null for unsupported optional columns. */
    private fun addRow(cursor: MatrixCursor, values: Map<String, Any?>) {
        cursor.addRow(cursor.columnNames.map { values[it] }.toTypedArray())
    }

    /** Identifies directories and maps unknown file extensions to the generic binary MIME type. */
    private fun mimeType(file: File): String = if (file.isDirectory) Document.MIME_TYPE_DIR else
        MimeTypeMap.getSingleton().getMimeTypeFromExtension(file.extension.lowercase(Locale.ROOT))
            ?: "application/octet-stream"

    /** Notifies both metadata readers and directory-list clients after a successful mutation. */
    private fun notifyChange(root: DocumentFileStore, documentId: String, parent: File?) {
        val resolver = providerContext().contentResolver
        resolver.notifyChange(DocumentsContract.buildDocumentUri(authority(), documentId), null)
        if (parent != null) {
            resolver.notifyChange(DocumentsContract.buildChildDocumentsUri(authority(), root.documentId(parent)), null)
        }
    }

    /** Obtains the attached Android context without starting any application service. */
    private fun providerContext() = checkNotNull(context) { "Document provider is not attached" }

    /** Uses each concrete provider's package-qualified authority. */
    protected abstract fun authority(): String

    companion object {
        private val ROOT_COLUMNS = arrayOf(Root.COLUMN_ROOT_ID, Root.COLUMN_DOCUMENT_ID, Root.COLUMN_TITLE,
            Root.COLUMN_SUMMARY, Root.COLUMN_MIME_TYPES, Root.COLUMN_ICON, Root.COLUMN_FLAGS, Root.COLUMN_AVAILABLE_BYTES)
        private val DOCUMENT_COLUMNS = arrayOf(Document.COLUMN_DOCUMENT_ID, Document.COLUMN_DISPLAY_NAME,
            Document.COLUMN_MIME_TYPE, Document.COLUMN_FLAGS, Document.COLUMN_SIZE, Document.COLUMN_LAST_MODIFIED)
    }
}

/** Keeps private application data accessible even when bootstrap configuration or JNI is broken. */
class OperitDataDocumentsProvider : OperitDocumentsProvider() {
    /** Publishes the application data directory, including startup records and maintenance files. */
    override fun roots(): List<DocumentFileStore> {
        val context = checkNotNull(context)
        return listOf(DocumentFileStore("private", File(context.applicationInfo.dataDir),
            context.getString(R.string.operit_documents_private), AndroidDocumentLinks::isSymbolicLink))
    }

    /** Returns the authority dedicated to the bootstrap-independent private data tree. */
    override fun authority(): String = "${checkNotNull(context).packageName}.data.documents"
}

/** Publishes the selected runtime and workspace bases, including every identity. */
class OperitStorageDocumentsProvider : OperitDocumentsProvider() {
    /** Reads only the Host's fixed bootstrap file without loading JNI or taking a runtime lease. */
    override fun roots(): List<DocumentFileStore> {
        val context = checkNotNull(context)
        val bootstrap = File(context.filesDir, "client/link/local_runtime_storage.json")
        if (!bootstrap.exists()) return emptyList()
        val config = JSONObject(bootstrap.readText(Charsets.UTF_8))
        if (!config.getBoolean("confirmed")) return emptyList()
        return listOf(
            DocumentFileStore.configured("runtime", File(config.getString("runtimeRoot")),
                context.getString(R.string.operit_documents_runtime), AndroidDocumentLinks::isSymbolicLink),
            DocumentFileStore.configured("workspaces", File(config.getString("workspaceRoot")),
                context.getString(R.string.operit_documents_workspaces), AndroidDocumentLinks::isSymbolicLink),
        )
    }

    /** Returns the authority dedicated to user-selected storage roots. */
    override fun authority(): String = "${checkNotNull(context).packageName}.storage.documents"
}

/** Supplies Android's no-follow metadata operation on every supported SDK version. */
internal object AndroidDocumentLinks {
    /** Detects even dangling links, and distinguishes a missing creation target from an I/O failure. */
    fun isSymbolicLink(file: File): Boolean = try {
        OsConstants.S_ISLNK(Os.lstat(file.path).st_mode)
    } catch (error: ErrnoException) {
        if (error.errno != OsConstants.ENOENT) throw error
        false
    }
}

/** Opens the system file manager directly at the independent private-data SAF root. */
internal object OperitDocumentManagement {
    /** Refreshes system root discovery after saved storage locations or restored data change. */
    fun notifyRootsChanged(context: Context) {
        context.contentResolver.notifyChange(
            DocumentsContract.buildRootsUri("${context.packageName}.storage.documents"), null,
        )
    }

    /** Launches document management without requiring a configured runtime or a rescue session. */
    fun open(activity: Activity) {
        notifyRootsChanged(activity)
        val picker = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
        }
        val manager = checkNotNull(activity.packageManager.resolveActivity(picker,
            PackageManager.MATCH_DEFAULT_ONLY or PackageManager.MATCH_SYSTEM_ONLY)) {
            "The system document manager is unavailable"
        }
        activity.startActivity(Intent(Intent.ACTION_VIEW).apply {
            // Operit's own attachment VIEW filter must not intercept document management.
            setPackage(manager.activityInfo.packageName)
            setDataAndType(DocumentsContract.buildRootUri("${activity.packageName}.data.documents", "private"),
                Root.MIME_TYPE_ITEM)
        })
    }
}

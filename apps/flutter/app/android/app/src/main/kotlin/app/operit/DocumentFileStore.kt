package app.operit

import java.io.File
import java.io.FileNotFoundException
import java.security.MessageDigest

/** Maps SAF document IDs to one published directory without following symbolic links. */
internal class DocumentFileStore(
    val id: String,
    directory: File,
    val title: String,
    private val isSymbolicLink: (File) -> Boolean,
) {
    val directory: File = directory.canonicalFile
    val documentId: String = "$id:"

    companion object {
        /** Binds grants to the selected directory so changing storage cannot retarget old URIs. */
        fun configured(kind: String, directory: File, title: String, isSymbolicLink: (File) -> Boolean): DocumentFileStore {
            require(directory.isAbsolute) { "Document roots must be absolute" }
            val canonical = directory.canonicalFile
            val digest = MessageDigest.getInstance("SHA-256").digest(canonical.path.toByteArray(Charsets.UTF_8))
                .joinToString("") { "%02x".format(it) }
            return DocumentFileStore("$kind-$digest", canonical, title, isSymbolicLink)
        }
    }

    /** Resolves an existing document and rejects traversal, aliases, and escaped links. */
    fun file(documentId: String): File {
        if (!documentId.startsWith("$id:")) throw FileNotFoundException("Unknown document root")
        val relative = documentId.substring(id.length + 1)
        var target = directory
        if (relative.isNotEmpty()) relative.split('/').forEach { name ->
            validateName(name)
            target = File(target, name)
            if (isSymbolicLink(target)) throw FileNotFoundException("Document path contains a symbolic link")
        }
        if (target.canonicalFile != target.absoluteFile ||
            (target != directory && !target.path.startsWith(directory.path.trimEnd(File.separatorChar) + File.separator))) {
            throw FileNotFoundException("Document is outside its published directory or is a symbolic link")
        }
        if (!target.exists()) throw FileNotFoundException("Document does not exist: $documentId")
        return target
    }

    /** Produces a relative identifier only for a validated document under this root. */
    fun documentId(file: File): String {
        val relative = if (file == directory) "" else file.relativeTo(directory).invariantSeparatorsPath
        val result = "$id:$relative"
        file(result)
        return result
    }

    /** Lists regular files and directories, excluding symbolic links from the document tree. */
    fun children(parentId: String): List<File> {
        val parent = file(parentId)
        if (!parent.isDirectory) throw FileNotFoundException("Document is not a directory")
        val children = parent.listFiles() ?: throw FileNotFoundException("Cannot list directory: $parentId")
        return children.filter { !isSymbolicLink(it) && (it.isDirectory || it.isFile) }
            .sortedWith(compareBy<File> { !it.isDirectory }.thenBy { it.name.lowercase(java.util.Locale.ROOT) })
    }

    /** Creates exactly the requested entry without overwriting an existing file. */
    fun create(parentId: String, name: String, directory: Boolean): String {
        validateName(name)
        val parent = file(parentId)
        if (!parent.isDirectory) throw FileNotFoundException("Document is not a directory")
        val child = File(parent, name)
        if (isSymbolicLink(child)) throw FileNotFoundException("Cannot replace a symbolic link")
        val created = if (directory) child.mkdir() else child.createNewFile()
        if (!created) throw FileNotFoundException("Cannot create document; the name may already exist")
        return documentId(child)
    }

    /** Renames a non-root document without replacing another entry. */
    fun rename(documentId: String, name: String): String {
        validateName(name)
        val source = mutableFile(documentId)
        val destination = File(source.parentFile, name)
        if (source == destination) return documentId
        if (destination.exists() || isSymbolicLink(destination)) {
            throw FileNotFoundException("The destination already exists or is a symbolic link")
        }
        if (!source.renameTo(destination)) throw FileNotFoundException("Cannot rename document")
        return documentId(destination)
    }

    /** Deletes a document tree while unlinking nested symbolic links without traversing them. */
    fun delete(documentId: String) {
        deleteEntry(mutableFile(documentId))
    }

    /** Requires a non-root document before an operation can remove its directory entry. */
    private fun mutableFile(documentId: String): File {
        val target = file(documentId)
        if (target == directory) throw FileNotFoundException("Cannot rename or delete a published root")
        return target
    }

    /** Removes children only while the entry remains a real directory. */
    private fun deleteEntry(file: File) {
        if (!isSymbolicLink(file) && file.isDirectory) {
            val children = file.listFiles() ?: throw FileNotFoundException("Cannot list directory for deletion")
            children.forEach(::deleteEntry)
        }
        if (!file.delete()) throw FileNotFoundException("Cannot delete document: ${file.name}")
    }

    /** Accepts a single path component without control characters or platform separators. */
    private fun validateName(name: String) {
        require(name.isNotEmpty() && name != "." && name != ".." &&
            name.none { it == '/' || it == '\\' || it.code < 32 }) { "Invalid document name" }
    }
}

package app.operit

import android.content.Context
import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileNotFoundException
import java.io.InputStream
import java.io.OutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream

/** Application-layer mounts: document IDs stay opaque and every access uses a tree grant.
 * Invoked by native runtime workers, never by the UI thread through MethodChannel. */
class AndroidDocumentFileSystem(
    private val documents: DocumentTreeAccess,
    private val fileOpener: ((File?, String?, String?) -> Unit)? = null,
) {
    constructor(context: Context) : this(
        ContentResolverDocumentTreeAccess(context), AndroidFileOpener(context)::open,
    )
    private val prefix = "operit-resource:"
    private class Missing(message: String) : FileNotFoundException(message)

    fun execute(encoded: String): String = try {
        val request = JSONObject(encoded)
        val value: Any? = when (request.getString("operation")) {
            "listFiles" -> JSONArray(node(request.getString("path")).children().map { it.entry() })
            "readFileBytes", "readFileWithLimit" -> {
                val maximum = request.optLong("maxBytes", -1)
                val bytes = node(request.getString("path")).input().use { input ->
                    if (maximum < 0) input.readBytes() else readBounded(input, maximum)
                }
                Base64.encodeToString(bytes, Base64.NO_WRAP)
            }
            "writeFileBytes", "writeFile" -> {
                val content = Base64.decode(request.getString("content"), Base64.DEFAULT)
                node(request.getString("path")).output(request.optBoolean("append")).use { it.write(content) }
                JSONObject.NULL
            }
            "openFile" -> {
                val target = node(request.getString("path"))
                require(!target.info().getBoolean("isDirectory")) { "Cannot open a directory as a file" }
                // Validate the real read capability, including revoked SAF grants.
                target.input().use { }
                val opener = fileOpener ?: throw UnsupportedOperationException("File opener is not installed")
                opener(target.file, target.tree, if (target.file == null) target.documentId() else null)
                JSONObject.NULL
            }
            "fileExists" -> node(request.getString("path")).existence()
            "fileInfo" -> node(request.getString("path")).info()
            "makeDirectory" -> {
                node(request.getString("path")).ensure(true, request.optBoolean("createParents"))
                JSONObject.NULL
            }
            "deleteFile" -> {
                node(request.getString("path")).delete(request.optBoolean("recursive"))
                JSONObject.NULL
            }
            "copyFile", "moveFile" -> {
                val source = node(request.getString("source"))
                val destination = node(request.getString("destination"))
                require(!destination.isWithin(source)) { "Cannot copy or move a mount into itself" }
                if (request.getString("operation") == "moveFile")
                    require(source.file != null || source.relative.isNotEmpty()) { "Cannot move a document mount root" }
                copy(source, destination, request.optBoolean("recursive"))
                // A resource move is explicitly copy-then-delete, never an atomic POSIX rename.
                if (request.getString("operation") == "moveFile") source.delete(true)
                JSONObject.NULL
            }
            "findFiles" -> find(request)
            "grepCode" -> grep(request)
            "zipFiles" -> {
                val source = node(request.getString("source"))
                val destination = node(request.getString("destination"))
                require(!destination.isWithin(source)) { "Archive destination must be outside its source" }
                ZipOutputStream(destination.output(false)).use { zip -> archive(source, "", zip, 0, HashSet()) }
                JSONObject.NULL
            }
            "unzipFiles" -> {
                val source = node(request.getString("source"))
                val destination = node(request.getString("destination"))
                require(!source.isWithin(destination)) { "Archive source must be outside its extraction directory" }
                destination.ensure(true, true)
                ZipInputStream(source.input()).use { zip ->
                    while (true) {
                        val entry = zip.nextEntry ?: break
                        val parts = relativeParts(entry.name.trimEnd('/'))
                        require(parts.isNotEmpty()) { "Empty archive entry" }
                        var target = destination
                        parts.forEach { target = target.child(it) }
                        require(target.isWithin(destination)) { "Archive entry escapes extraction directory" }
                        if (entry.isDirectory) target.ensure(true, true)
                        else target.output(false).use { zip.copyTo(it) }
                        zip.closeEntry()
                    }
                }
                JSONObject.NULL
            }
            else -> throw UnsupportedOperationException("Unsupported document filesystem operation")
        }
        JSONObject().put("ok", true).put("value", value ?: JSONObject.NULL).toString()
    } catch (error: Exception) {
        JSONObject().put("ok", false).put("error", "${error.javaClass.simpleName}: ${error.message}").toString()
    }

    private fun relativeParts(path: String): List<String> {
        require(!path.startsWith('/') && !path.contains('\\') && !path.contains('\u0000')) { "Invalid relative resource path" }
        if (path.isEmpty()) return emptyList()
        val parts = path.split('/')
        require(parts.all { it.isNotEmpty() && it != "." && it != ".." }) { "Invalid relative resource path" }
        return parts
    }

    private fun node(target: String): Node {
        if (!target.startsWith(prefix)) {
            val file = File(target)
            require(file.isAbsolute) { "Native paths must be absolute" }
            return Node(null, "", file)
        }
        val resource = JSONObject(target.substring(prefix.length))
        require(resource.getString("backend") == "android_documents") { "Unsupported Android filesystem backend" }
        val tree = resource.getString("root")
        if (!documents.hasReadGrant(tree)) {
            throw SecurityException("Document tree permission was revoked or is missing; select this folder again")
        }
        val relative = resource.getString("path")
        relativeParts(relative)
        return Node(tree, relative, null)
    }

    private inner class Node(val tree: String?, val relative: String, val file: File?, val knownId: String? = null) {
        fun child(name: String): Node {
            require(relativeParts(name).size == 1) { "Invalid child name" }
            return if (file != null) Node(null, "", File(file, name))
            else Node(tree, if (relative.isEmpty()) name else "$relative/$name", null)
        }
        fun parent(): Node {
            if (file != null) return Node(null, "", file.parentFile ?: throw IllegalArgumentException("Root has no parent"))
            require(relative.isNotEmpty()) { "Cannot modify a document mount root" }
            return Node(tree, relative.substringBeforeLast('/', ""), null)
        }
        fun name(): String = file?.name ?: relative.substringAfterLast('/')
        fun locator(): String = file?.absolutePath ?: prefix + JSONObject()
            .put("backend", "android_documents").put("root", tree.toString()).put("path", relative).toString()
        fun isWithin(other: Node): Boolean {
            if (file != null && other.file != null) {
                val mine = file.canonicalPath
                val theirs = other.file.canonicalPath.trimEnd('/')
                return mine == theirs || mine.startsWith("$theirs/")
            }
            return tree != null && tree == other.tree && (relative == other.relative ||
                other.relative.isEmpty() || relative.startsWith("${other.relative}/"))
        }
        private fun writable() {
            if (tree != null && !documents.hasWriteGrant(tree))
                throw SecurityException("Selected document tree is read-only")
        }
        fun documentId(): String {
            knownId?.let { return it }
            var current = documents.rootId(tree!!)
            for (part in relativeParts(relative)) {
                current = findChild(tree, current, part) ?: throw Missing("Document does not exist: $relative")
            }
            return current
        }
        fun ensure(directory: Boolean, parents: Boolean): String? {
            if (file != null) {
                if (file.exists()) {
                    require(file.isDirectory == directory) { "Existing target has a different file type" }
                    return null
                }
                if (directory) {
                    require(if (parents) file.mkdirs() else file.mkdir()) { "Cannot create directory" }
                } else {
                    if (parents) require(file.parentFile?.isDirectory == true || file.parentFile?.mkdirs() == true) { "Cannot create parent directory" }
                    require(file.createNewFile()) { "Cannot create file" }
                }
                return null
            }
            writable()
            var current = documents.rootId(tree!!)
            val parts = relativeParts(relative)
            if (parts.isEmpty()) {
                require(directory) { "Document mount root is a directory" }
                return current
            }
            for ((index, part) in parts.withIndex()) {
                val existing = findChild(tree, current, part)
                val isDirectory = index < parts.lastIndex || directory
                if (existing != null) {
                    require(documentInfo(tree, existing).getBoolean("isDirectory") == isDirectory) { "Existing document has a different file type" }
                    current = existing
                } else {
                    if (index < parts.lastIndex && !parents) throw Missing("Parent directory does not exist")
                    require(documents.describe(tree, current).writable) { "Provider does not support creating children in this directory" }
                    current = documents.create(tree, current, part, isDirectory)
                }
            }
            return current
        }
        fun stat(): JSONObject {
            if (file != null) {
                if (!file.exists()) throw Missing("File does not exist: $file")
                return JSONObject().put("name", file.name).put("isDirectory", file.isDirectory)
                    .put("size", if (file.isDirectory) 0 else file.length())
                    .put("permissions", (if (file.canRead()) "r" else "-") + (if (file.canWrite()) "w" else "-"))
                    .put("lastModified", file.lastModified().toString())
            }
            return documentInfo(tree!!, documentId())
        }
        fun entry(): JSONObject = stat()
        fun existence(): JSONObject = try {
            val data = stat()
            JSONObject().put("exists", true).put("isDirectory", data.getBoolean("isDirectory")).put("size", data.getLong("size"))
        } catch (_: Missing) { JSONObject().put("exists", false).put("isDirectory", false).put("size", 0) }
        fun info(): JSONObject {
            val data = stat()
            return JSONObject().put("path", locator()).put("exists", true)
                .put("fileType", if (data.getBoolean("isDirectory")) "directory" else "file")
                .put("size", data.getLong("size")).put("permissions", data.getString("permissions"))
                .put("owner", "").put("group", "").put("lastModified", data.getString("lastModified")).put("rawStatOutput", "")
        }
        fun children(): List<Node> {
            require(stat().getBoolean("isDirectory")) { "Not a directory" }
            if (file != null) return (file.listFiles() ?: throw IllegalStateException("Cannot list directory"))
                .map { Node(null, "", it) }.sortedBy { it.name() }
            return documents.children(tree!!, documentId()).map { metadata ->
                val child = child(metadata.name)
                Node(tree, child.relative, null, metadata.id)
            }.sortedBy { it.name() }
        }

        fun input(): InputStream {
            require(!stat().getBoolean("isDirectory")) { "Cannot read a directory" }
            return file?.inputStream() ?: documents.input(tree!!, documentId())
        }
        fun output(append: Boolean): OutputStream {
            writable()
            val created = ensure(false, true)
            if (tree != null) require(documents.describe(tree, created!!).writable) { "Provider reports this document as read-only" }
            return file?.outputStreamFor(append) ?: documents.output(tree!!, created!!, append)
        }
        fun isNativeSymlink(): Boolean = file?.parentFile?.let { parent ->
            File(parent.canonicalFile, file.name).absolutePath != file.canonicalPath
        } ?: false
        fun delete(recursive: Boolean, depth: Int = 0, seen: MutableSet<String> = HashSet()) {
            require(depth <= 128 && seen.add(identity())) { "Deletion encountered a cycle or excessive depth" }
            writable()
            require(file != null || relative.isNotEmpty()) { "Cannot delete a document mount root" }
            if (tree != null) require(documents.describe(tree, documentId()).deletable) { "Provider does not support deleting this document" }
            if (!isNativeSymlink() && stat().getBoolean("isDirectory")) {
                val children = children()
                require(recursive || children.isEmpty()) { "Directory is not empty; recursive deletion is required" }
                if (recursive) children.forEach { it.delete(true, depth + 1, seen) }
            }
            if (file != null) require(file.delete()) { "Could not delete $file" }
            else require(documents.delete(tree!!, documentId())) { "Provider could not delete document" }
        }
        fun identity(): String = file?.canonicalPath ?: JSONObject().put("tree", tree).put("id", documentId()).toString()
    }

    private fun File.outputStreamFor(append: Boolean): OutputStream = java.io.FileOutputStream(this, append)

    private fun findChild(tree: String, parentId: String, name: String): String? {
        val children = documents.children(tree, parentId).filter { it.name == name }
        require(children.size <= 1) { "Provider contains ambiguous duplicate document names: $name" }
        return children.firstOrNull()?.id
    }

    private fun documentInfo(tree: String, id: String): JSONObject {
        val metadata = try { documents.describe(tree, id) }
        catch (error: FileNotFoundException) { throw Missing(error.message ?: "Document does not exist") }
        return JSONObject().put("name", metadata.name).put("isDirectory", metadata.directory)
            .put("size", metadata.size).put("lastModified", metadata.modified)
            .put("permissions", if (metadata.writable) "rw" else "r-")
    }

    private fun readBounded(input: InputStream, maximum: Long): ByteArray {
        require(maximum in 0..Int.MAX_VALUE.toLong()) { "Invalid read size limit" }
        val output = java.io.ByteArrayOutputStream()
        val buffer = ByteArray(64 * 1024)
        var remaining = maximum
        while (remaining > 0) {
            val count = input.read(buffer, 0, minOf(buffer.size.toLong(), remaining).toInt())
            if (count < 0) break
            output.write(buffer, 0, count)
            remaining -= count
        }
        return output.toByteArray()
    }

    private fun copy(source: Node, destination: Node, recursive: Boolean, depth: Int = 0, seen: MutableSet<String> = HashSet()) {
        require(!source.isNativeSymlink()) { "Copying native symbolic links through document mounts is unsupported" }
        require(depth <= 128 && seen.add(source.identity())) { "Recursive copy encountered a cycle or excessive depth" }
        if (source.stat().getBoolean("isDirectory")) {
            require(recursive) { "Copying a directory requires recursive=true" }
            destination.ensure(true, true)
            source.children().forEach { copy(it, destination.child(it.name()), true, depth + 1, seen) }
        } else source.input().use { input -> destination.output(false).use { input.copyTo(it) } }
    }

    private fun glob(pattern: String, caseInsensitive: Boolean): Regex {
        val out = StringBuilder("^")
        var i = 0
        while (i < pattern.length) {
            when (val char = pattern[i]) {
                '*' -> { if (i + 1 < pattern.length && pattern[i + 1] == '*') { out.append(".*"); i++ } else out.append("[^/]*") }
                '?' -> out.append("[^/]")
                else -> out.append(Regex.escape(char.toString()))
            }
            i++
        }
        out.append('$')
        return Regex(out.toString(), if (caseInsensitive) setOf(RegexOption.IGNORE_CASE) else emptySet())
    }

    private fun walk(root: Node, maximum: Int, visit: (Node, String) -> Unit) {
        val seen = HashSet<String>()
        fun descend(current: Node, relative: String, depth: Int) {
            require(depth <= 128) { "Search exceeded the maximum document depth" }
            if (current.isNativeSymlink() || !seen.add(current.identity())) return
            visit(current, relative)
            if (current.stat().getBoolean("isDirectory") && (maximum < 0 || depth < maximum))
                current.children().forEach { descend(it, if (relative.isEmpty()) it.name() else "$relative/${it.name()}", depth + 1) }
        }
        descend(root, "", 0)
    }
    private fun find(request: JSONObject): JSONArray {
        val root = node(request.getString("path"))
        val pattern = glob(request.getString("pattern"), request.optBoolean("caseInsensitive"))
        val matches = JSONArray()
        walk(root, request.optInt("maxDepth", -1)) { current, relative ->
            if (relative.isNotEmpty() && pattern.matches(if (request.optBoolean("usePathPattern")) relative else current.name()))
                matches.put(current.locator())
        }
        return matches
    }
    private fun grep(request: JSONObject): JSONObject {
        val pattern = Regex(request.getString("pattern"), if (request.optBoolean("caseInsensitive")) setOf(RegexOption.IGNORE_CASE) else emptySet())
        val filter = glob(request.optString("filePattern", "*").ifEmpty { "*" }, false)
        val matches = JSONArray()
        val maximum = request.optInt("maxResults", 100).coerceAtLeast(0)
        val context = request.optInt("contextLines", 0).coerceIn(0, 1000)
        var total = 0
        var searched = 0
        walk(node(request.getString("path")), -1) { current, relative ->
            if (total < maximum && !current.stat().getBoolean("isDirectory") && (filter.matches(current.name()) || filter.matches(relative))) {
                // Bound individual search reads so a provider cannot exhaust the runtime heap.
                val bytes = current.input().use { readBounded(it, 8L * 1024 * 1024 + 1) }
                require(bytes.size <= 8 * 1024 * 1024) { "Document exceeds the 8 MiB content-search limit: ${current.name()}" }
                val text = bytes.toString(Charsets.UTF_8)
                searched++
                if (!text.contains('\u0000')) {
                    val lines = text.lines()
                    val hits = JSONArray()
                    for ((index, line) in lines.withIndex()) if (total < maximum && pattern.containsMatchIn(line)) {
                        val hit = JSONObject().put("lineNumber", index + 1).put("lineContent", line)
                        hit.put("matchContext", if (context == 0) JSONObject.NULL else lines.subList(maxOf(0, index - context), minOf(lines.size, index + context + 1)).joinToString("\n"))
                        hits.put(hit)
                        total++
                    }
                    if (hits.length() > 0) matches.put(JSONObject().put("filePath", current.locator()).put("lineMatches", hits))
                }
            }
        }
        return JSONObject().put("matches", matches).put("totalMatches", total).put("filesSearched", searched)
    }
    private fun archive(source: Node, relative: String, zip: ZipOutputStream, depth: Int, seen: MutableSet<String>) {
        require(!source.isNativeSymlink()) { "Archiving native symbolic links through document mounts is unsupported" }
        require(depth <= 128 && seen.add(source.identity())) { "Archive source contains a cycle or excessive depth" }
        if (source.stat().getBoolean("isDirectory")) {
            if (relative.isNotEmpty()) { zip.putNextEntry(ZipEntry("$relative/")); zip.closeEntry() }
            source.children().forEach { archive(it, if (relative.isEmpty()) it.name() else "$relative/${it.name()}", zip, depth + 1, seen) }
        } else {
            zip.putNextEntry(ZipEntry(relative.ifEmpty { source.name() }))
            source.input().use { it.copyTo(zip) }
            zip.closeEntry()
        }
    }
}

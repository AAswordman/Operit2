package app.operit

import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileNotFoundException
import java.io.InputStream
import java.io.OutputStream
import java.nio.file.Files
import java.util.UUID
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

/** JVM contract suite: opaque document IDs, persisted grants, and filesystem operations.
 * Run with the real AndroidDocumentFileSystem + Android API jar, no device required. */
private class FakeDocuments : DocumentTreeAccess {
    data class Item(val id: String, var name: String, val directory: Boolean, val parent: String?, var data: ByteArray = byteArrayOf())
    val tree = "content://test.documents/tree/opaque%3Aroot"
    val items = linkedMapOf("opaque-root" to Item("opaque-root", "Project", true, null))
    var readGrant = true
    var writeGrant = true
    var failWrites = false
    fun metadata(item: Item) = DocumentMetadata(item.id, item.name, item.directory, item.data.size.toLong(), "123", writeGrant)
    fun get(id: String) = items[id] ?: throw FileNotFoundException("Missing opaque ID")
    override fun hasReadGrant(tree: String) = tree == this.tree && readGrant
    override fun hasWriteGrant(tree: String) = tree == this.tree && writeGrant
    override fun rootId(tree: String) = "opaque-root"
    override fun children(tree: String, parentId: String) = items.values.filter { it.parent == parentId }.map(::metadata)
    override fun describe(tree: String, documentId: String) = metadata(get(documentId))
    override fun create(tree: String, parentId: String, name: String, directory: Boolean): String {
        check(writeGrant && get(parentId).directory)
        val id = "opaque:${UUID.randomUUID()}/../not-a-path"
        items[id] = Item(id, name, directory, parentId)
        return id
    }
    override fun delete(tree: String, documentId: String): Boolean {
        check(writeGrant)
        check(items.values.none { it.parent == documentId })
        return items.remove(documentId) != null
    }
    override fun input(tree: String, documentId: String): InputStream = ByteArrayInputStream(get(documentId).data)
    override fun output(tree: String, documentId: String, append: Boolean): OutputStream {
        check(writeGrant && !failWrites) { "Simulated provider write failure" }
        val item = get(documentId)
        return object : ByteArrayOutputStream() {
            override fun close() {
                item.data = (if (append) item.data else byteArrayOf()) + toByteArray()
                super.close()
            }
        }
    }
    fun target(path: String) = "operit-resource:" + JSONObject().put("backend", "android_documents").put("root", tree).put("path", path)
}

private var assertions = 0
private fun verify(value: Boolean, message: String) { check(value) { message }; assertions++ }

fun main() {
    val docs = FakeDocuments()
    val opened = mutableListOf<Triple<File?, String?, String?>>()
    var openFailure = false
    val fs = AndroidDocumentFileSystem(docs) { file, tree, id ->
        check(!openFailure) { "Viewer denied opening" }
        opened.add(Triple(file, tree, id))
    }
    val temporary = Files.createTempDirectory("operit-saf-filesystem").toFile()
    fun invoke(operation: String, vararg fields: Pair<String, Any>): JSONObject = JSONObject(fs.execute(
        JSONObject().put("operation", operation).also { request -> fields.forEach { request.put(it.first, it.second) } }.toString()))
    fun success(operation: String, vararg fields: Pair<String, Any>): Any {
        val response = invoke(operation, *fields)
        verify(response.getBoolean("ok"), "$operation: $response")
        return response.get("value")
    }
    fun failure(operation: String, vararg fields: Pair<String, Any>): String {
        val response = invoke(operation, *fields)
        verify(!response.getBoolean("ok"), "Expected $operation to fail: $response")
        return response.getString("error")
    }
    fun write(path: String, bytes: ByteArray, append: Boolean = false) = success("writeFile", "path" to path,
        "content" to Base64.encodeToString(bytes, Base64.NO_WRAP), "append" to append)
    fun read(path: String) = Base64.decode(success("readFileBytes", "path" to path) as String, Base64.DEFAULT)
    try {
        val text = "第一行\nhello\nhello again\n"
        write(docs.target("src/项目 file.py"), text.toByteArray())
        verify(read(docs.target("src/项目 file.py")).contentEquals(text.toByteArray()), "Unicode document read/write")
        docs.writeGrant = false
        success("openFile", "path" to docs.target("src/项目 file.py"))
        docs.writeGrant = true
        verify(opened.single().second == docs.tree, "Opener retains the persisted tree grant")
        verify(opened.single().third == docs.items.values.single { it.name == "项目 file.py" }.id,
            "Opener receives the opaque document ID, not a fabricated filesystem path")
        failure("openFile", "path" to docs.target("src"))
        failure("openFile", "path" to docs.target("missing.txt"))
        docs.readGrant = false
        failure("openFile", "path" to docs.target("src/项目 file.py"))
        docs.readGrant = true
        val native = File(temporary, "中文 & 100%.txt").also { it.writeText("native file") }
        success("openFile", "path" to native.path)
        verify(opened.last() == Triple(native, null, null), "Native files reach the same platform opener")
        failure("openFile", "path" to temporary.path)
        openFailure = true
        verify(failure("openFile", "path" to native.path).contains("Viewer denied"), "Viewer failures are returned")
        openFailure = false
        val listing = success("listFiles", "path" to docs.target("src")) as JSONArray
        verify(listing.getJSONObject(0).getString("name") == "项目 file.py", "Child display names")
        verify(docs.items.values.single { it.name == "项目 file.py" }.id != "src/项目 file.py", "IDs remain opaque")
        val limited = Base64.decode(success("readFileWithLimit", "path" to docs.target("src/项目 file.py"), "maxBytes" to 3) as String, Base64.DEFAULT)
        verify(limited.size == 3, "Bounded reads")
        write(docs.target("binary.dat"), byteArrayOf(0, 1, -1))
        write(docs.target("binary.dat"), byteArrayOf(2), true)
        verify(read(docs.target("binary.dat")).contentEquals(byteArrayOf(0, 1, -1, 2)), "Binary append does not overwrite")
        failure("makeDirectory", "path" to docs.target("missing/child"), "createParents" to false)
        success("makeDirectory", "path" to docs.target("empty"), "createParents" to true)
        success("deleteFile", "path" to docs.target("empty"), "recursive" to false)
        failure("deleteFile", "path" to docs.target("src"), "recursive" to false)
        verify(read(docs.target("src/项目 file.py")).isNotEmpty(), "Nonrecursive delete preserves contents")

        docs.writeGrant = false
        success("listFiles", "path" to docs.target(""))
        verify(failure("writeFile", "path" to docs.target("no.txt"), "content" to "").contains("read-only"), "Read-only grant failure")
        docs.writeGrant = true
        docs.readGrant = false
        verify(failure("fileExists", "path" to docs.target("binary.dat")).contains("revoked"), "Revocation is not false existence")
        docs.readGrant = true
        val missing = success("fileExists", "path" to docs.target("missing.txt")) as JSONObject
        verify(!missing.getBoolean("exists"), "Missing document existence")
        for (path in listOf("../escape", "a/../escape", "/absolute", "a\\b")) failure("fileExists", "path" to docs.target(path))

        val found = success("findFiles", "path" to docs.target(""), "pattern" to "src/*.PY", "usePathPattern" to true, "caseInsensitive" to true, "maxDepth" to 2) as JSONArray
        verify(found.length() == 1 && found.getString(0).startsWith("operit-resource:"), "Find returns resource locators")
        val grep = success("grepCode", "path" to docs.target("src"), "pattern" to "hello", "filePattern" to "*.py", "maxResults" to 1, "contextLines" to 1) as JSONObject
        verify(grep.getInt("totalMatches") == 1, "Grep result limit")
        val line = grep.getJSONArray("matches").getJSONObject(0).getJSONArray("lineMatches").getJSONObject(0)
        verify(line.getInt("lineNumber") == 2 && line.getString("matchContext").contains("第一行"), "Grep context and line numbers")

        val native = File(temporary, "copy.py")
        success("copyFile", "source" to docs.target("src/项目 file.py"), "destination" to native.path, "recursive" to false)
        verify(native.readText() == text, "SAF to native copy")
        success("copyFile", "source" to native.path, "destination" to docs.target("copied/native.py"), "recursive" to false)
        verify(read(docs.target("copied/native.py")).toString(Charsets.UTF_8) == text, "Native to SAF copy")
        success("copyFile", "source" to docs.target("src"), "destination" to docs.target("recursive"), "recursive" to true)
        verify(read(docs.target("recursive/项目 file.py")).isNotEmpty(), "Recursive SAF copy")
        failure("copyFile", "source" to docs.target("src"), "destination" to docs.target("src/nested"), "recursive" to true)
        write(docs.target("move.txt"), "move me".toByteArray())
        docs.failWrites = true
        failure("moveFile", "source" to docs.target("move.txt"), "destination" to docs.target("failed-move.txt"), "recursive" to true)
        verify(read(docs.target("move.txt")).isNotEmpty(), "Failed copy does not delete move source")
        docs.failWrites = false
        success("moveFile", "source" to docs.target("move.txt"), "destination" to docs.target("moved.txt"), "recursive" to true)
        verify(!(success("fileExists", "path" to docs.target("move.txt")) as JSONObject).getBoolean("exists"), "Move deletes source after success")
        failure("moveFile", "source" to docs.target(""), "destination" to File(temporary, "root-copy").path, "recursive" to true)
        verify(!File(temporary, "root-copy").exists(), "Mount-root move rejected before copying")

        val archive = File(temporary, "project.zip")
        success("zipFiles", "source" to docs.target("src"), "destination" to archive.path)
        success("unzipFiles", "source" to archive.path, "destination" to docs.target("extracted"))
        verify(read(docs.target("extracted/项目 file.py")).toString(Charsets.UTF_8) == text, "SAF archive round trip")
        for (entry in listOf("../outside", "/absolute", "dir\\outside")) {
            val malicious = File(temporary, "malicious.zip")
            ZipOutputStream(malicious.outputStream()).use { zip -> zip.putNextEntry(ZipEntry(entry)); zip.write(1); zip.closeEntry() }
            failure("unzipFiles", "source" to malicious.path, "destination" to File(temporary, "unzip").path)
            verify(!File(temporary, "outside").exists(), "Zip traversal blocked")
        }
        val outside = File(temporary, "outside-dir").also { it.mkdir() }
        val extraction = File(temporary, "symlink-unzip").also { it.mkdir() }
        Files.createSymbolicLink(File(extraction, "link").toPath(), outside.toPath())
        val malicious = File(temporary, "symlink.zip")
        ZipOutputStream(malicious.outputStream()).use { zip -> zip.putNextEntry(ZipEntry("link/escape.txt")); zip.write(1); zip.closeEntry() }
        failure("unzipFiles", "source" to malicious.path, "destination" to extraction.path)
        verify(!File(outside, "escape.txt").exists(), "Symlink zip traversal blocked")

        val duplicate1 = docs.create(docs.tree, docs.rootId(docs.tree), "duplicate", false)
        val duplicate2 = docs.create(docs.tree, docs.rootId(docs.tree), "duplicate", false)
        failure("readFileBytes", "path" to docs.target("duplicate"))
        docs.items.remove(duplicate1); docs.items.remove(duplicate2)
        success("deleteFile", "path" to docs.target("recursive"), "recursive" to true)
        verify(!(success("fileExists", "path" to docs.target("recursive")) as JSONObject).getBoolean("exists"), "Recursive deletion")
        failure("deleteFile", "path" to docs.target(""), "recursive" to true)
        println("Android SAF filesystem contracts passed: $assertions assertions")
    } finally { temporary.deleteRecursively() }
}

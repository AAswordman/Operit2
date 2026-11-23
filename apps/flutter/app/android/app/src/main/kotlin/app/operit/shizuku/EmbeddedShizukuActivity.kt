package app.operit.shizuku

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.text.Html
import android.view.Menu
import android.view.MenuItem
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import app.operit.embeddedshizuku.R
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import rikka.shizuku.Shizuku
import java.util.concurrent.Executors

/** Hosts Shizuku's copied home cards and pairing flow inside the Operit APK. */
class EmbeddedShizukuActivity : ShizukuAppBarActivity() {
    private val worker = Executors.newSingleThreadExecutor()
    private lateinit var cards: RecyclerView
    private val binderListener = Shizuku.OnBinderReceivedListener { runOnUiThread { refresh() } }
    private val deathListener = Shizuku.OnBinderDeadListener { runOnUiThread { refresh() } }
    private val permissionListener = Shizuku.OnRequestPermissionResultListener { request, grant ->
        if (request == 2408) runOnUiThread {
            if (grant == 0) refresh() else showError(SecurityException("外部 Shizuku 拒绝授权"))
        }
    }

    /** Inflates upstream XML without recreating its card shapes, typography or controls. */
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        title = "Shizuku"
        supportActionBar?.setDisplayHomeAsUpEnabled(true)
        setContentView(R.layout.home_activity)
        cards = findViewById(android.R.id.list)
        val layouts = listOf(R.layout.home_server_status, R.layout.home_start_wireless_adb,
            R.layout.home_start_root, R.layout.home_start_adb)
        cards.adapter = object : RecyclerView.Adapter<CardHolder>() {
            /** Inflates the original rounded container and its original card body. */
            override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): CardHolder {
                val card = layoutInflater.inflate(R.layout.home_item_container, parent, false) as ViewGroup
                val body = layoutInflater.inflate(viewType, card, false)
                card.addView(body)
                (card.layoutParams as ViewGroup.MarginLayoutParams).apply {
                    val side = (16 * resources.displayMetrics.density).toInt()
                    setMargins(side, side / 4, side, side / 4)
                }
                return CardHolder(card, body)
            }
            /** Binds only the activation callbacks to Operit's embedded service. */
            override fun onBindViewHolder(holder: CardHolder, position: Int) = bindCard(holder.body, layouts[position])
            /** Preserves the source layout as the view type. */
            override fun getItemViewType(position: Int): Int = layouts[position]
            /** Returns the owner-only subset of Shizuku's home cards. */
            override fun getItemCount(): Int = layouts.size
        }
        Shizuku.addBinderReceivedListenerSticky(binderListener)
        Shizuku.addBinderDeadListener(deathListener)
        Shizuku.addRequestPermissionResultListener(permissionListener)
    }

    /** Holds the copied card body independently of RecyclerView recycling. */
    private class CardHolder(root: View, val body: View) : RecyclerView.ViewHolder(root)

    /** Keeps the original wireless guide, pairing and start controls on the same card. */
    private fun bindCard(body: View, layout: Int) {
        when (layout) {
            R.layout.home_server_status -> {
                val connected = Shizuku.pingBinder()
                val name = if (EmbeddedShizuku.selected(this)) "内置 Shizuku" else "外部 Shizuku"
                body.findViewById<TextView>(android.R.id.text1).text = getString(
                    if (connected) R.string.home_status_service_is_running else R.string.home_status_service_not_running, name)
                body.findViewById<TextView>(android.R.id.text2).text = if (connected)
                    "API ${Shizuku.getVersion()} · UID ${Shizuku.getUid()} · ${if (Shizuku.checkSelfPermission() == 0) "已授权" else "未授权"}"
                    else "Operit 内置系统服务"
                body.findViewById<ImageView>(android.R.id.icon).setImageResource(
                    if (connected) R.drawable.ic_server_ok_24dp else R.drawable.ic_server_error_24dp)
            }
            R.layout.home_start_wireless_adb -> {
                body.findViewById<TextView>(android.R.id.text1).text = Html.fromHtml(
                    getString(R.string.home_wireless_adb_description), Html.FROM_HTML_MODE_LEGACY).trim()
                for (id in listOf(android.R.id.button2, android.R.id.button3)) body.findViewById<View>(id).setOnClickListener {
                    if (Build.VERSION.SDK_INT >= 30) startActivity(Intent(this, EmbeddedShizukuPairingActivity::class.java))
                    else showError(IllegalStateException("无线配对需要 Android 11 或更高版本"))
                }
                body.findViewById<View>(android.R.id.button1).setOnClickListener {
                    runOperation("正在搜索并启动内置 Shizuku") { EmbeddedShizuku.discoverAndStart(this) }
                }
            }
            R.layout.home_start_root -> {
                body.findViewById<TextView>(android.R.id.text1).text = "使用 Root 激活内置服务。服务及普通命令以 shell 身份运行。"
                body.findViewById<View>(android.R.id.button1).apply {
                    isEnabled = true
                    setOnClickListener { runOperation("正在启动内置 Shizuku") { EmbeddedShizuku.startRoot(this@EmbeddedShizukuActivity) } }
                }
            }
            R.layout.home_start_adb -> {
                body.findViewById<TextView>(android.R.id.text1).text = "连接电脑后执行 ADB 指令，激活同一 APK 内的系统服务。"
                body.findViewById<View>(android.R.id.button1).setOnClickListener {
                    EmbeddedShizuku.selectComputerActivation(this)
                    val command = "adb shell ${EmbeddedShizuku.command(this)}"
                    MaterialAlertDialogBuilder(this).setTitle(R.string.home_adb_button_view_command).setMessage(command)
                        .setPositiveButton(R.string.home_adb_dialog_view_command_copy_button) { _, _ ->
                            getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("ADB", command))
                        }.setNegativeButton(android.R.string.cancel, null).show()
                    refresh()
                }
            }
        }
    }

    /** Places recovery and external backend selection behind the original toolbar settings icon. */
    override fun onCreateOptionsMenu(menu: Menu): Boolean {
        menu.add("设置").setIcon(R.drawable.ic_settings_outline_24dp).setShowAsAction(MenuItem.SHOW_AS_ACTION_ALWAYS)
        return true
    }

    /** Keeps secondary configuration out of the original home activation flow. */
    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        if (item.itemId == android.R.id.home) return super.onOptionsItemSelected(item)
        val recovery = EmbeddedShizuku.restoreEnabled(this)
        MaterialAlertDialogBuilder(this).setTitle("系统服务设置").setItems(arrayOf(
            "开机自动恢复：${if (recovery) "已开启" else "已关闭"}", "停止内置服务", "使用外部 Shizuku", "使用 Sui", "请求外部 Shizuku 授权")) { _, index ->
            when (index) {
                0 -> MaterialAlertDialogBuilder(this).setTitle(if (recovery) "关闭自动恢复" else "开启自动恢复")
                    .setMessage("无需 Root。无线激活会保存配对身份、关闭 ADB 授权超时，并在开机时恢复调试开关。关闭后还原原先的授权超时设置。")
                    .setPositiveButton(android.R.string.ok) { _, _ -> runOperation("正在设置恢复") { EmbeddedShizuku.setRestore(this, !recovery) } }
                    .setNegativeButton(android.R.string.cancel, null).show()
                1 -> runOperation("正在停止") { EmbeddedShizuku.stop(this) }
                2 -> runOperation("正在切换") { EmbeddedShizuku.selectExternal(this) }
                3 -> runOperation("正在连接 Sui") { EmbeddedShizuku.selectSui(this) }
                4 -> if (!EmbeddedShizuku.selected(this) && Shizuku.pingBinder()) Shizuku.requestPermission(2408)
                    else showError(IllegalStateException("请先连接外部 Shizuku"))
            }
        }.show()
        return true
    }

    /** Serializes activation while showing a Material progress dialog instead of editable ports. */
    private fun runOperation(title: String, operation: () -> Unit) {
        val dialog = MaterialAlertDialogBuilder(this).setTitle(title).setMessage("正在处理…").setCancelable(false).show()
        worker.execute {
            val error = runCatching(operation).exceptionOrNull()
            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                dialog.dismiss()
                if (error != null) showError(error)
                refresh()
            }
        }
    }

    /** Preserves the activation error without replacing the service's actual status. */
    private fun showError(error: Throwable) {
        MaterialAlertDialogBuilder(this).setTitle("操作未完成").setMessage(error.message ?: error.javaClass.simpleName)
            .setPositiveButton(android.R.string.ok, null).show()
    }

    /** Refreshes copied status cards after a real Binder or lifecycle transition. */
    private fun refresh() { if (::cards.isInitialized) cards.adapter?.notifyDataSetChanged() }

    /** Reads current state after returning from the system pairing screen. */
    override fun onResume() { super.onResume(); refresh() }

    /** Releases listeners without stopping an independently running system service. */
    override fun onDestroy() {
        Shizuku.removeBinderReceivedListener(binderListener)
        Shizuku.removeBinderDeadListener(deathListener)
        Shizuku.removeRequestPermissionResultListener(permissionListener)
        worker.shutdown()
        super.onDestroy()
    }
}

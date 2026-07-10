package app.operit

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

object AndroidHostSecretStore {
    private const val ANDROID_KEYSTORE = "AndroidKeyStore"
    private const val KEY_ALIAS = "Operit2.HostSecretStore"
    private const val PREFERENCES_NAME = "operit_host_secret_store"
    private const val DATA_SUFFIX = ".data"
    private const val IV_SUFFIX = ".iv"
    private const val GCM_TAG_BITS = 128
    private const val BASE64_FLAGS = Base64.NO_WRAP or Base64.URL_SAFE

    /** Reads host secret bytes from Android secure storage. */
    fun read(context: Context, key: String): ByteArray? {
        val preferences = preferences(context)
        val preferenceKey = encodedKey(key)
        val ivText = preferences.getString(preferenceKey + IV_SUFFIX, null)
        val dataText = preferences.getString(preferenceKey + DATA_SUFFIX, null)
        if (ivText == null && dataText == null) {
            return null
        }
        check(ivText != null && dataText != null) {
            "Host secret record is incomplete for key: $key"
        }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(
            Cipher.DECRYPT_MODE,
            secretKey(),
            GCMParameterSpec(GCM_TAG_BITS, base64Decode(ivText)),
        )
        return cipher.doFinal(base64Decode(dataText))
    }

    /** Writes host secret bytes into Android secure storage. */
    fun write(context: Context, key: String, content: ByteArray) {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, secretKey())
        val encrypted = cipher.doFinal(content)
        val preferenceKey = encodedKey(key)
        val persisted =
            preferences(context)
                .edit()
                .putString(preferenceKey + IV_SUFFIX, base64Encode(cipher.iv))
                .putString(preferenceKey + DATA_SUFFIX, base64Encode(encrypted))
                .commit()
        check(persisted) {
            "Host secret write did not persist for key: $key"
        }
    }

    /** Deletes host secret bytes from Android secure storage. */
    fun delete(context: Context, key: String) {
        val preferenceKey = encodedKey(key)
        val persisted =
            preferences(context)
                .edit()
                .remove(preferenceKey + IV_SUFFIX)
                .remove(preferenceKey + DATA_SUFFIX)
                .commit()
        check(persisted) {
            "Host secret deletion did not persist for key: $key"
        }
    }

    /** Returns the preferences file used for encrypted host secret payloads. */
    private fun preferences(context: Context): SharedPreferences {
        return context.getSharedPreferences(PREFERENCES_NAME, Context.MODE_PRIVATE)
    }

    /** Encodes a host secret key into a stable preferences key. */
    private fun encodedKey(key: String): String {
        return base64Encode(key.toByteArray(Charsets.UTF_8))
    }

    /** Encodes bytes with URL-safe base64 for preferences storage. */
    private fun base64Encode(bytes: ByteArray): String {
        return Base64.encodeToString(bytes, BASE64_FLAGS)
    }

    /** Decodes URL-safe base64 bytes from preferences storage. */
    private fun base64Decode(value: String): ByteArray {
        return Base64.decode(value, BASE64_FLAGS)
    }

    /** Returns the Android KeyStore key used to encrypt host secrets. */
    private fun secretKey(): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
        keyStore.load(null)
        val existing = loadSecretKey(keyStore)
        if (existing != null) {
            return existing
        }
        return createSecretKey()
    }

    /** Loads the existing Android KeyStore key. */
    private fun loadSecretKey(keyStore: KeyStore): SecretKey? {
        val key = keyStore.getKey(KEY_ALIAS, null) ?: return null
        check(key is SecretKey) {
            "Android KeyStore alias is not a secret key: $KEY_ALIAS"
        }
        return key
    }

    /** Creates the Android KeyStore key used for host secret encryption. */
    private fun createSecretKey(): SecretKey {
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        val spec =
            KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .setUserAuthenticationRequired(false)
                .build()
        generator.init(spec)
        return generator.generateKey()
    }
}

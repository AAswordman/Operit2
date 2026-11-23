// Modified by Operit: isolate the embedded owner from the external Shizuku manager.
package rikka.shizuku.server;

public class ServerConstants {

    public static final int MANAGER_APP_NOT_FOUND = 50;

    public static final String MANAGER_APPLICATION_ID = System.getProperty("operit.shizuku.manager", "app.operit");
}

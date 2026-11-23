package app.operit.embeddedshizuku;

import android.os.Build;
import android.os.IBinder;
import java.lang.reflect.InvocationTargetException;
import java.util.ArrayList;
import java.util.List;

/** Resolves framework method signatures without linking obsolete return types into DEX. */
public final class SystemUsers {
    /** Obtains a framework Binder interface in the service's shell identity. */
    private static Object service(String name, String stub) throws ReflectiveOperationException {
        IBinder binder = (IBinder) Class.forName("android.os.ServiceManager")
            .getMethod("getService", String.class).invoke(null, name);
        return Class.forName(stub).getMethod("asInterface", IBinder.class).invoke(null, binder);
    }

    /** Uses the user-enumeration signature actually provided by each Android generation. */
    public static List<Integer> users() {
        try {
            Object manager = service("user", "android.os.IUserManager$Stub");
            Class<?> api = Class.forName("android.os.IUserManager");
            List<?> users;
            if (Build.VERSION.SDK_INT >= 37 || Build.VERSION.SDK_INT < 30) {
                users = (List<?>) api.getMethod("getUsers", boolean.class).invoke(manager, true);
            } else {
                users = (List<?>) api.getMethod("getUsers", boolean.class, boolean.class, boolean.class)
                    .invoke(manager, true, true, true);
            }
            List<Integer> ids = new ArrayList<>();
            for (Object user : users) ids.add(user.getClass().getField("id").getInt(user));
            return ids;
        } catch (ReflectiveOperationException error) {
            throw failure("enumerate users", error);
        }
    }

    /** Preserves the real framework error instead of reporting an empty successful result. */
    private static IllegalStateException failure(String operation, ReflectiveOperationException error) {
        Throwable cause = error instanceof InvocationTargetException ? error.getCause() : error;
        return new IllegalStateException("Cannot " + operation, cause);
    }
}

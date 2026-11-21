package app.operit.windowserver;

interface IUiServer {
    int getProtocolVersion();
    void attach(IBinder owner, IBinder windowSession);
    String execute(String requestJson);
}

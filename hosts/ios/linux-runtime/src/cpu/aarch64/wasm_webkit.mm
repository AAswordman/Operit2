#import <Foundation/Foundation.h>
#import <WebKit/WebKit.h>
#include "cpu/aarch64/wasm_executor.h"
#include <atomic>
#include <memory>
#include <mutex>
#include <unordered_map>
#include <unordered_set>
// Only misses cross this channel. Cached branches, calls and returns remain in
// WebContent and invoke directly compiled Wasm functions over one resident heap.
@interface ORTWasmHost : NSObject <WKNavigationDelegate,WKURLSchemeHandler>
@property(nonatomic,strong) WKWebView *view;
@property(nonatomic,strong) NSData *input;
@property(nonatomic,strong) NSMutableSet<id<WKURLSchemeTask>> *tasks;
@property(nonatomic,copy) void (^ready)(BOOL);
@property(nonatomic,copy) void (^request)(NSData *,BOOL,void (^)(NSData *));
@property(nonatomic,copy) void (^terminated)(void);
@end
@implementation ORTWasmHost
- (void)webView:(WKWebView *)view startURLSchemeTask:(id<WKURLSchemeTask>)task {
    (void)view;
    if(!self.tasks)self.tasks=[NSMutableSet new];[self.tasks addObject:task];
    void (^reply)(NSData *)=^(NSData *data) {
        if(![self.tasks containsObject:task])return;
        [self.tasks removeObject:task];
        BOOL page=[task.request.URL.path isEqualToString:@"/index.html"];
        NSURLResponse *response=[[NSHTTPURLResponse alloc] initWithURL:task.request.URL statusCode:200 HTTPVersion:@"HTTP/1.1"
            headerFields:@{@"Content-Type":page ? @"text/html":@"application/octet-stream",@"Cache-Control":@"no-store"}];
        [task didReceiveResponse:response];[task didReceiveData:data ?: [NSData data]];[task didFinish];
    };
    NSString *path=task.request.URL.path;
    if([path isEqualToString:@"/index.html"]) {reply([@"<!doctype html>" dataUsingEncoding:NSUTF8StringEncoding]);return;}
    if([path isEqualToString:@"/input"]) {reply(self.input);self.input=nil;return;}
    NSData *body=task.request.HTTPBody;
    if(!body && task.request.HTTPBodyStream) {
        NSMutableData *data=[NSMutableData new];NSInputStream *stream=task.request.HTTPBodyStream;
        [stream open];uint8_t buffer[4096];NSInteger n;
        while((n=[stream read:buffer maxLength:sizeof(buffer)])>0)[data appendBytes:buffer length:n];
        [stream close];body=data;
    }
    if(self.request && body)self.request(body,[path isEqualToString:@"/final"],reply);
    else { [self.tasks removeObject:task];[task didFailWithError:[NSError errorWithDomain:@"OperitWasm" code:1 userInfo:nil]]; }
}
- (void)webView:(WKWebView *)view stopURLSchemeTask:(id<WKURLSchemeTask>)task {(void)view;[self.tasks removeObject:task];}
- (void)webView:(WKWebView *)view didFinishNavigation:(WKNavigation *)navigation {
    (void)view;(void)navigation;auto ready=self.ready;self.ready=nil;if(ready)ready(YES);
}
- (void)webView:(WKWebView *)view didFailNavigation:(WKNavigation *)navigation withError:(NSError *)error {
    (void)view;(void)navigation;(void)error;auto ready=self.ready;self.ready=nil;if(ready)ready(NO);
}
- (void)webViewWebContentProcessDidTerminate:(WKWebView *)view {
    (void)view;auto terminated=self.terminated;self.view=nil;auto ready=self.ready;self.ready=nil;if(ready)ready(NO);if(terminated)terminated();
}
@end
namespace operit::arm64 {
namespace {
struct Conversation {
    dispatch_semaphore_t wake=dispatch_semaphore_create(0);
    NSData *message=nil;
    void (^respond)(NSData *)=nil;
    std::atomic<bool> finished{false},failed{false};
};
struct Session {
    std::mutex mutex;
    std::atomic<bool> terminated{false};
    std::unordered_set<uint64_t> installed;
    ORTWasmHost *host=nil;
    ~Session() {
        ORTWasmHost *saved=host;
        if(saved)dispatch_async(dispatch_get_main_queue(),^{
            saved.request=nil;saved.ready=nil;saved.terminated=nil;saved.input=nil;saved.tasks=nil;saved.view=nil;
        });
    }
};
std::mutex sessions_mutex;
std::unordered_map<uint64_t,std::shared_ptr<Session>> sessions;
NSData *outgoing(WasmTransfer &t,std::unordered_set<uint64_t> &installed,bool initial=false) {
    const bool upload=t.module && installed.insert(t.module_id).second;
    static const auto dispatcher=WasmRegion::dispatcher();
    if(t.module_id>UINT32_MAX)throw Failure("Wasm module table exhausted");
    const uint32_t header[]={uint32_t(t.reason),uint32_t(t.stop),uint32_t(t.module_id),
        upload ? uint32_t(t.module->entries.size()):0u,uint32_t(t.pages.size()),uint32_t(t.invalidate.size()),
        upload ? uint32_t(t.module->bytes.size()):0u,initial ? uint32_t(dispatcher.size()):0u};
    NSMutableData *data=[NSMutableData dataWithCapacity:sizeof(header)+t.state.size()+t.pages.size()*4112u+header[6]+header[7]+header[3]*12u+header[5]*4u];
    [data appendBytes:header length:sizeof(header)];[data appendBytes:t.state.data() length:t.state.size()];
    if(upload) {
        for(auto [pc,selector]:t.module->entries) {const uint32_t entry[]={uint32_t(pc),uint32_t(pc>>32),selector};[data appendBytes:entry length:sizeof(entry)];}
        ++t.module_uploads;t.module_bytes+=t.module->bytes.size();
    }
    for(auto slot:t.invalidate)[data appendBytes:&slot length:sizeof(slot)];
    for(auto &p:t.pages) {
        const uint32_t page[]={p.slot,uint32_t(p.address),uint32_t(p.address>>32),p.permissions};
        [data appendBytes:page length:sizeof(page)];[data appendBytes:p.bytes.data() length:p.bytes.size()];
    }
    if(upload)[data appendBytes:t.module->bytes.data() length:t.module->bytes.size()];
    if(initial)[data appendBytes:dispatcher.data() length:dispatcher.size()];
    return data;
}
void incoming(NSData *message,WasmTransfer &t) {
    constexpr size_t header_size=32,metrics_size=7*sizeof(double);
    if(message.length<header_size+WasmRegion::state_size+metrics_size)throw Failure("invalid Wasm transport frame");
    const auto *p=static_cast<const uint8_t *>(message.bytes);uint32_t header[4];double wasm;
    std::memcpy(header,p,sizeof(header));std::memcpy(&wasm,p+16,sizeof(wasm));
    if(message.length!=header_size+WasmRegion::state_size+size_t(header[2])*4100+metrics_size)throw Failure("invalid Wasm page frame");
    t.state.assign(p+header_size,p+header_size+WasmRegion::state_size);p+=header_size+WasmRegion::state_size;
    t.pages.clear();t.invalidate.clear();t.module.reset();
    for(uint32_t n=0;n<header[2];++n) {
        WasmTransfer::Page page;std::memcpy(&page.slot,p,4);p+=4;page.bytes.assign(p,p+4096);p+=4096;t.pages.push_back(std::move(page));
    }
    t.reason=header[0];t.stop=header[1];t.wasm_retired=uint64_t(wasm);t.dispatches=header[3];
}
// One WebView executes one guest owner at a time. Nothing waits on the main
// thread: requests are serviced by the original runtime worker while JS awaits.
NSString *const source=@R"JS(
if(!window.executor)window.executor={memory:new WebAssembly.Memory({initial:37}),entries:new Map(),modules:new Map(),pages:new Map(),table:new WebAssembly.Table({element:'anyfunc',initial:1}),next:2424832};
const e=window.executor;
const key=(lo,hi)=>hi+':'+lo;
let bytes,view,retired=0,wasm=0,dispatches=0,chains=0,fmaCalls=0;
let executeMs=0,installMs=0,snapshotMs=0,fmaMs=0,bridgeMs=0;
const stamp=()=>profile ? performance.now():0;
const refresh=()=>{bytes=new Uint8Array(e.memory.buffer);view=new DataView(e.memory.buffer)};
refresh();
const install=async buffer=>{
    const started=stamp();
    const frame=new DataView(buffer),raw=new Uint8Array(buffer),reason=frame.getUint32(0,true),stop=frame.getUint32(4,true),module=frame.getUint32(8,true);
    const entries=frame.getUint32(12,true),pages=frame.getUint32(16,true),invalid=frame.getUint32(20,true),binary=frame.getUint32(24,true),dispatcher=frame.getUint32(28,true);
    const state=raw.subarray(32,1056),entryOffset=1056;let pos=entryOffset+entries*12;
    for(let n=0;n<invalid;n++,pos+=4){const table=65536+frame.getUint32(pos,true)*16;view.setUint32(table,0,true);view.setUint32(table+12,0,true)}
    for(let n=0;n<pages;n++){
        const slot=frame.getUint32(pos,true),lo=frame.getUint32(pos+4,true),hi=frame.getUint32(pos+8,true),permissions=frame.getUint32(pos+12,true);pos+=16;
        let page=e.pages.get(slot),offset=page ? page.offset:e.next;
        if(!page){e.next+=4096;if(e.next>bytes.length){e.memory.grow(Math.ceil((e.next-bytes.length)/65536));refresh()}}
        bytes.set(raw.subarray(pos,pos+4096),offset);pos+=4096;const table=65536+slot*16;
        view.setUint32(table,lo|permissions,true);view.setUint32(table+4,hi,true);view.setUint32(table+8,offset,true);view.setUint32(table+12,permissions,true);
        e.pages.set(slot,{offset});
    }
    if(!e.dispatcher && dispatcher)e.dispatcher=(await WebAssembly.instantiate(raw.subarray(pos+binary,pos+binary+dispatcher),{env:{memory:e.memory,table:e.table}})).instance.exports.run;
    if(module){
        let instance=e.modules.get(module);
        if(!instance){if(!binary)throw new Error('missing Wasm module');instance=(await WebAssembly.instantiate(raw.subarray(pos,pos+binary),{env:{memory:e.memory,table:e.table,fma,fma32}})).instance;e.modules.set(module,instance);if(module>=e.table.length)e.table.grow(module+1-e.table.length);e.table.set(module,instance.exports.run)}
        for(let n=0;n<entries;n++){
            const entry=entryOffset+n*12,lo=frame.getUint32(entry,true),hi=frame.getUint32(entry+4,true),selector=frame.getUint32(entry+8,true);
            e.entries.set(key(lo,hi),{selector,module});
            const slot=((lo>>>2)^((lo>>>18)|(hi<<14)))&65535,table=1114112+slot*16;
            view.setUint32(table,lo,true);view.setUint32(table+4,hi,true);view.setUint32(table+8,module,true);view.setUint32(table+12,selector,true);
        }
    }
    bytes.set(state,0);retired=view.getUint32(268,true);installMs+=stamp()-started;return {reason,stop};
};
const snapshot=(reason,stop)=>{
    const started=stamp(),count=view.getUint32(416,true);
    view.setUint32(268,retired,true);view.setUint32(416,0,true);
    const packet=new ArrayBuffer(32+1024+count*4100+56),out=new Uint8Array(packet),frame=new DataView(packet);
    frame.setUint32(0,reason,true);frame.setUint32(4,stop ? 1:0,true);frame.setUint32(8,count,true);frame.setUint32(12,dispatches,true);frame.setFloat64(16,wasm,true);
    out.set(bytes.subarray(0,1024),32);let pos=1056;
    for(let n=0;n<count;n++){const slot=view.getUint32(2162688+n*4,true),table=65536+slot*16,offset=view.getUint32(table+8,true);
        view.setUint32(table+12,view.getUint32(table+12,true)&~8,true);frame.setUint32(pos,slot,true);pos+=4;out.set(bytes.subarray(offset,offset+4096),pos);pos+=4096}
    snapshotMs+=stamp()-started;
    for(const n of [chains,fmaCalls,executeMs,installMs,snapshotMs,fmaMs,bridgeMs]){frame.setFloat64(pos,n,true);pos+=8}
    return packet;
};
const exchange=async (path,body)=>{const response=await fetch('operit-wasm://executor/'+path,{method:'POST',body});if(!response.ok)throw new Error('Wasm transport failed');return response.arrayBuffer()};
// Numerical FMA helper: exact integer product/sum and one nearest-even
// rounding. It handles arithmetic only; no guest opcodes or addresses enter JS.
const scratch=new DataView(new ArrayBuffer(8));
const parts=x=>{scratch.setFloat64(0,x,true);const lo=scratch.getUint32(0,true),hi=scratch.getUint32(4,true),ex=(hi>>>20)&2047;
    return {m:(BigInt(hi&1048575)*4294967296n+BigInt(lo)+(ex ? 4503599627370496n:0n))*(hi>>>31 ? -1n:1n),e:(ex||1)-1075}};
const fused=(a,b,c,precision)=>{
    const x=parts(a),y=parts(b),z=parts(c),ep=x.e+y.e,e=Math.min(ep,z.e);
    let n=(x.m*y.m<<BigInt(ep-e))+(z.m<<BigInt(z.e-e));
    if(n===0n)return Object.is(a*b,-0)&&Object.is(c,-0) ? -0:0;
    const sign=n<0n;if(sign)n=-n;let bits=n.toString(2).length,ex=e+bits-1;
    if(ex < (precision===53 ? -1022:-126) || ex>(precision===53 ? 1023:127))return NaN;
    if(bits>precision){const shift=BigInt(bits-precision),rest=n&((1n<<shift)-1n),half=1n<<(shift-1n);n>>=shift;
        if(rest>half || (rest===half && (n&1n)))n++;if(n===(1n<<BigInt(precision))){n>>=1n;ex++}}
    else n<<=BigInt(precision-bits);
    if(precision===24)return (sign ? -1:1)*Number(n)*2**(ex-23);
    if(ex>1023)return NaN;
    scratch.setUint32(0,Number(n&4294967295n),true);
    scratch.setUint32(4,(sign ? 2147483648:0)+((ex+1023)*1048576)+Number((n>>32n)&1048575n),true);
    return scratch.getFloat64(0,true);
};
const fusedProfile=(a,b,c,p)=>{const started=stamp();++fmaCalls;const r=fused(a,b,c,p);fmaMs+=stamp()-started;return r};
e.fused=fusedProfile;
const fma=(a,b,c)=>e.fused(a,b,c,53),fma32=(a,b,c)=>e.fused(a,b,c,24);
await install(await (await fetch('operit-wasm://executor/input',{cache:'no-store'})).arrayBuffer());
const finish=async reason=>{await exchange('final',snapshot(reason,true));return 0};
for(;;){
    const remaining=view.getUint32(272,true);
    if(!remaining)return finish(0);
    const target=e.entries.get(key(view.getUint32(256,true),view.getUint32(260,true)));
    let reason=1;
    if(target){
        // Restore this entry after a direct-map collision, then let Wasm's
        // shared-table trampoline connect all cached regions without JS calls.
        const lo=view.getUint32(256,true),hi=view.getUint32(260,true),slot=((lo>>>2)^((lo>>>18)|(hi<<14)))&65535,table=1114112+slot*16;
        view.setUint32(table,lo,true);view.setUint32(table+4,hi,true);view.setUint32(table+8,target.module,true);view.setUint32(table+12,target.selector,true);
        view.setUint32(388,0,true);const started=stamp();reason=e.dispatcher();executeMs+=stamp()-started;++chains;dispatches+=view.getUint32(388,true);
        const n=view.getUint32(268,true);retired+=n;wasm+=n;
        if(reason===1)continue;
        if(reason===3 || !view.getUint32(272,true))return finish(reason);
        // A block bigger than the remaining exact budget needs native suffix
        // execution; it must not repeatedly re-enter the same unadmitted block.
    }
    const message=snapshot(reason,false),started=stamp();
    const command=await exchange('step',message);bridgeMs+=stamp()-started;
    const result=await install(command);if(result.stop)return finish(result.reason);
}
)JS";
}
bool wasm_webkit_available() {return !NSThread.isMainThread;}
void wasm_webkit_forget(uint64_t session) {
    std::shared_ptr<Session> removed;
    {std::lock_guard<std::mutex> lock(sessions_mutex);auto found=sessions.find(session);
        if(found!=sessions.end()) {removed=std::move(found->second);sessions.erase(found);}}
}
bool wasm_webkit_run(uint64_t session,WasmTransfer &transfer,const std::function<void(WasmTransfer&)> &service) {
    if(!wasm_webkit_available())return false;
    @autoreleasepool {
        std::shared_ptr<Session> owner;
        {std::lock_guard<std::mutex> guard(sessions_mutex);auto &entry=sessions[session];if(!entry)entry=std::make_shared<Session>();owner=entry;}
        std::lock_guard<std::mutex> lock(owner->mutex);
        if(owner->terminated.load())return false;
        auto &installed=owner->installed;
        auto conversation=std::make_shared<Conversation>();NSData *input=outgoing(transfer,installed,true);
        transfer.pages.clear();transfer.invalidate.clear();transfer.module.reset();
        dispatch_async(dispatch_get_main_queue(),^{
            ORTWasmHost *host=owner->host;if(!host)owner->host=host=[ORTWasmHost new];
            std::weak_ptr<Session> weak=owner;
            host.terminated=^{
                if(auto current=weak.lock())current->terminated.store(true);
                if(!conversation->finished) {conversation->failed=conversation->finished=true;dispatch_semaphore_signal(conversation->wake);}
            };
            host.input=input;
            host.request=^(NSData *message,BOOL final,void (^respond)(NSData *)) {
                conversation->message=message;
                if(final)respond(nil);
                else {conversation->respond=respond;dispatch_semaphore_signal(conversation->wake);}
            };
            auto execute=^(BOOL ready) {
                if(conversation->finished)return;
                if(!ready){conversation->failed=conversation->finished=true;dispatch_semaphore_signal(conversation->wake);return;}
                [host.view callAsyncJavaScript:source arguments:@{@"profile":@(std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_OUTPUT")!=nullptr)}
                    inFrame:nil inContentWorld:WKContentWorld.pageWorld completionHandler:^(id result,NSError *error) {
                        (void)result;
                        if(conversation->finished)return;
                        conversation->failed=error!=nil || !conversation->message;conversation->finished=true;
                        if(error)NSLog(@"Wasm execution failed: %@",error);
                        host.request=nil;dispatch_semaphore_signal(conversation->wake);
                    }];
            };
            if(host.view)execute(YES);else {
                WKWebViewConfiguration *configuration=[WKWebViewConfiguration new];configuration.websiteDataStore=WKWebsiteDataStore.nonPersistentDataStore;
                [configuration setURLSchemeHandler:host forURLScheme:@"operit-wasm"];
                host.view=[[WKWebView alloc] initWithFrame:CGRectMake(0,0,1,1) configuration:configuration];host.view.navigationDelegate=host;host.ready=execute;
                [host.view loadRequest:[NSURLRequest requestWithURL:[NSURL URLWithString:@"operit-wasm://executor/index.html"]]];
            }
        });
        bool ok=true;
        for(;;) {
            dispatch_semaphore_wait(conversation->wake,DISPATCH_TIME_FOREVER);
            if(conversation->failed){ok=false;break;}
            try {incoming(conversation->message,transfer);service(transfer);}
            catch(const std::exception &error) {NSLog(@"Wasm transport failed: %s",error.what());transfer.stop=true;ok=false;}
            if(conversation->finished) {
                const auto *path=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_OUTPUT");
                if(path && *path) {
                    FILE *out=(!std::strcmp(path,"stderr") || !std::strcmp(path,"-")) ? stderr:std::fopen(path,"a");
                    double metrics[7];std::memcpy(metrics,static_cast<const uint8_t *>(conversation->message.bytes)+conversation->message.length-sizeof(metrics),sizeof(metrics));
                    if(out) {std::fprintf(out,"{\"kind\":\"wasm_webcontent\",\"wasm\":%llu,\"dispatches\":%llu,\"chains\":%.0f,\"fmaCalls\":%.0f,\"executeMs\":%.3f,\"installMs\":%.3f,\"snapshotMs\":%.3f,\"fmaMs\":%.3f,\"bridgeMs\":%.3f}\n",
                        (unsigned long long)transfer.wasm_retired,(unsigned long long)transfer.dispatches,metrics[0],metrics[1],metrics[2],metrics[3],metrics[4],metrics[5],metrics[6]);if(out!=stderr)std::fclose(out);}
                }
                break;
            }
            auto respond=conversation->respond;conversation->respond=nil;
            NSData *reply=outgoing(transfer,installed);
            transfer.pages.clear();transfer.invalidate.clear();transfer.module.reset();
            dispatch_async(dispatch_get_main_queue(),^{respond(reply);});
        }
        if(!ok)owner->terminated.store(true);
        return ok;
    }
}
}

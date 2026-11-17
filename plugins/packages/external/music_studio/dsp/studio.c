/* Fixed-memory, sample-free stereo DSP. No malloc, host imports, or audio-node creation.
 * ABI: write parameters into scratch(), then configure tracks/effects; render <=128 frames.
 * All sound generation AND all seven insert effects execute in WebAssembly.
 */
typedef unsigned int u32;
#define TRACKS 24
#define VOICES 64
#define FX 144
#define ARENA (8*1024*1024)
#define UNISON 8
#define PARAMS 36
#define HAAS_SIZE 8192
#define PI 3.14159265358979323846f
static float sr=48000, master=.72f, smMaster=.72f, mixAlpha=.004f, scratchpad[128], output[256];
static float arena[ARENA], sine[2049], frequencies[128];
static int used, trackCount, activeCount, droppedCount, maxActive;
static u32 randomState;
static float clamp(float x,float a,float b){return x<a?a:x>b?b:x;}
static float absf(float x){return x<0?-x:x;}
static float wrap(float x){return x-__builtin_floorf(x);}
static float sinp(float p){p=wrap(p)*2048;int i=(int)p;return sine[i]+(sine[i+1]-sine[i])*(p-i);}
static float exp2f_(float x){x=clamp(x,-32,24);int n=(int)__builtin_floorf(x);float f=(x-n)*.69314718056f;union{u32 i;float f;}v;v.i=(u32)(n+127)<<23;return v.f*(1+f*(1+f*(.5f+f*(.16666667f+f*(.04166667f+f*.00833333f)))));}
static float decayFactor(float seconds){return exp2f_(-1.442695f/(sr*seconds));}
static float noise(void){randomState^=randomState<<13;randomState^=randomState>>17;randomState^=randomState<<5;return (float)(randomState>>8)*(1.f/8388608.f)-1;}
static float sat(float x){x=clamp(x,-3,3);return x*(27+x*x)/(27+9*x*x);}
static float blep(float p,float dt){if(p<dt){p/=dt;return p+p-p*p-1;}if(p>1-dt){p=(p-1)/dt;return p*p+p+p+1;}return 0;}
/* Fade partials before Nyquist, never fold a fixed harmonic into the audible band. */
static float partial(float p,float dt,int harmonic){
 float f=dt*harmonic;if(f>=.48f)return 0;
 float u=clamp((f-.40f)/.08f,0,1),weight=1-u*u*(3-2*u);
 return sinp(p*harmonic)*weight;
}
static float osc(int type,float p,float dt){
 if(type==0)return sinp(p);
 if(type==1){float x=0;for(int j=0;j<8;j++){int h=2*j+1;x+=(j&1?-1.f:1.f)*partial(p,dt,h)/(h*h);}return (8.f/(PI*PI))*x;}
 if(type==2)return 2*p-1-blep(p,dt);
 if(type==3)return (p<.5f?1:-1)+blep(p,dt)-blep(wrap(p+.5f),dt);
 if(type==4)return .72f*sinp(p)+.16f*partial(p,dt,3)+.08f*partial(p,dt,7)+.04f*partial(p,dt,11);
 return .85f*sinp(p)+.11f*partial(p,dt,3)+.04f*partial(p,dt,5);
}
typedef struct{float z1,z2;} BState;
typedef struct{int type,next,offset,size,pos;float mix,p[24],state[12],phase;BState b[6];} Effect;
typedef struct{float p[PARAMS],det[16],pl[16],pr[16],level,pan,cutoff,smLevel,smPan,smCutoff,smGain,peak,lp[2],sideLow[2],sideAlpha;int first,last,haasPos;} Track;
typedef struct{int live,track,pitch,age,gate,release,attack,decay,total;BState filter[2];float phase[16],inc[16],coeff[5],coeffStep[5],env,amp,releaseStart,lp[4],fm,decayMul,noiseLow,drumPhase,subPhase,subInc;
 BState airFilter[6];float airCoeff[15],airStep[15],airMotion[2],airControl[4];u32 airRandom;} Voice;
static Track tracks[TRACKS]; static Voice voices[VOICES];static Effect effects[FX]; static int fxCount;
static float busL[TRACKS][128],busR[TRACKS][128],haasL[TRACKS][HAAS_SIZE],haasR[TRACKS][HAAS_SIZE];
__attribute__((export_name("scratch"))) float* scratch(void){return scratchpad;}
__attribute__((export_name("output"))) float* get_output(void){return output;}
__attribute__((export_name("active"))) int active(void){return activeCount;}
__attribute__((export_name("dropped"))) int dropped(void){return droppedCount;}
__attribute__((export_name("max_active"))) int max_active(void){return maxActive;}
__attribute__((export_name("track_peak"))) float track_peak(int t){return t>=0&&t<trackCount?tracks[t].peak:0;}
static void clear(void* p,int bytes){unsigned char* c=p;for(int i=0;i<bytes;i++)c[i]=0;}
__attribute__((export_name("init"))) void init(float rate,float gain){
 sr=rate;master=smMaster=gain;mixAlpha=1-exp2f_(-1.442695f/(sr*.005f));used=fxCount=trackCount=activeCount=droppedCount=maxActive=0;randomState=0x12345678;
 clear(tracks,sizeof(tracks));clear(voices,sizeof(voices));clear(effects,sizeof(effects));
 for(int i=0;i<=2048;i++){float x=(float)i/2048*2*PI; if(x>PI)x-=2*PI;if(x>PI*.5f)x=PI-x;else if(x<-PI*.5f)x=-PI-x;float x2=x*x;sine[i]=x*(1+x2*(-1.f/6+x2*(1.f/120+x2*(-1.f/5040+x2*(1.f/362880+x2*(-1.f/39916800))))));}
 sine[0]=sine[1024]=sine[2048]=0;sine[512]=1;sine[1536]=-1;
 for(int i=0;i<128;i++)frequencies[i]=440*exp2f_((i-69)/12.f);
}
static void update_spread(Track* tr){
 for(int bank=0;bank<2;bank++){
  int n=(int)clamp(tr->p[bank?22:5],1,UNISON);float detune=tr->p[bank?23:4],width=tr->p[bank?24:15];
  float tuning=bank?(tr->p[25]*12+tr->p[26]+tr->p[27]/100):0;
  for(int u=0;u<n;u++){int slot=bank*UNISON+u;float sp=n==1?0:2.f*u/(n-1)-1;
   float spread=sp*(.65f+.35f*sp*sp); /* Symmetric, center-weighted detuning, not a phase-locked linear comb. */
   tr->det[slot]=exp2f_((spread*detune/100+tuning)/12);
   tr->pl[slot]=__builtin_sqrtf((1-sp*width)*.5f);tr->pr[slot]=__builtin_sqrtf((1+sp*width)*.5f);
  }
 }
}
__attribute__((export_name("set_track"))) void set_track(int t){
 if(t<0||t>=TRACKS)return;Track* tr=&tracks[t];for(int i=0;i<PARAMS;i++)tr->p[i]=scratchpad[i];tr->first=tr->last=-1;
 tr->smGain=tr->p[20];tr->level=tr->smLevel=1;tr->pan=tr->smPan=tr->p[21];tr->cutoff=tr->smCutoff=18000;trackCount=t+1;
 clear(haasL[t],sizeof(haasL[t]));clear(haasR[t],sizeof(haasR[t]));tr->haasPos=0;tr->sideLow[0]=tr->sideLow[1]=0;
 tr->sideAlpha=1-exp2f_(-9.06472f*tr->p[35]/sr);
 update_spread(tr);

}
/* Live controls keep all phase/envelope/filter/effect state and transport position. */
__attribute__((export_name("set_master"))) void set_master(float gain){master=clamp(gain,0,1);}
__attribute__((export_name("set_gain"))) void set_gain(int t,float gain){if(t>=0&&t<trackCount)tracks[t].p[20]=clamp(gain,0,1.5f);}
__attribute__((export_name("update_track"))) void update_track(int t){
 if(t<0||t>=trackCount)return;Track* tr=&tracks[t];float previous[PARAMS];
 for(int i=0;i<PARAMS;i++){previous[i]=tr->p[i];tr->p[i]=scratchpad[i];}
 update_spread(tr);tr->pan=tr->p[21];tr->sideAlpha=1-exp2f_(-9.06472f*tr->p[35]/sr);
 for(int k=0;k<VOICES;k++){
  Voice* v=&voices[k];if(!v->live||v->track!=t)continue;
  for(int bank=0;bank<2;bank++)for(int u=0;u<(int)tr->p[bank?22:5];u++){
   int slot=bank*UNISON+u;
   if(u>=(int)previous[bank?22:5])v->phase[slot]=wrap(tr->p[31]+(noise()+1)*.5f*tr->p[32]);
   v->inc[slot]=clamp(frequencies[v->pitch]/sr*tr->det[slot],.000001f,.45f);
  }
  v->subInc=clamp(frequencies[v->pitch]/sr*exp2f_(tr->p[29]),.000001f,.45f);
  v->attack=(int)clamp(tr->p[6]*sr,1,v->gate*.45f);v->decay=(int)(tr->p[7]*sr);
  v->release=(int)(tr->p[9]*sr);v->decayMul=decayFactor(tr->p[7]*.3f);
  if(tr->p[0]!=3)v->total=v->gate+v->release;
 }
}
__attribute__((export_name("add_fx"))) int add_fx(int t,int type,float mix){
 if(t<0||t>=trackCount||fxCount>=FX)return -1;
 Effect* e=&effects[fxCount];e->type=type;e->mix=mix;e->next=-1;for(int i=0;i<24;i++)e->p[i]=scratchpad[i];
 int size=type==4?(int)e->p[2]:type==3?4096:type==5?16384:0; // 4 independent 4096-sample reverb lines
 if(used+size*2>ARENA)return -2;
 e->offset=used;e->size=size;clear(arena+used,size*2*sizeof(float));used+=size*2;
 if(tracks[t].last>=0)effects[tracks[t].last].next=fxCount;else tracks[t].first=fxCount;
 tracks[t].last=fxCount++;return 0;
}
__attribute__((export_name("update_fx"))) int update_fx(int f,float mix){
 if(f<0||f>=fxCount)return -1;Effect* e=&effects[f];
 if(e->type==4&&(int)scratchpad[2]>e->size)return -1;
 for(int i=0;i<24;i++)e->p[i]=scratchpad[i];e->mix=mix;return 0;
}
__attribute__((export_name("has_note"))) int has_note(int t,int pitch){for(int i=0;i<VOICES;i++)if(voices[i].live&&voices[i].track==t&&voices[i].pitch==pitch&&voices[i].age<voices[i].gate)return 1;return 0;}
__attribute__((export_name("automate"))) void automate(int t,float level,float pan,float cutoff){if(t<0||t>=trackCount)return;tracks[t].level=level;tracks[t].pan=pan;tracks[t].cutoff=cutoff;}
__attribute__((export_name("reset"))) void reset(void){clear(voices,sizeof(voices));activeCount=0;randomState=0x12345678;for(int t=0;t<trackCount;t++){tracks[t].lp[0]=tracks[t].lp[1]=0;tracks[t].smLevel=tracks[t].level;tracks[t].smPan=tracks[t].pan;tracks[t].smCutoff=tracks[t].cutoff;tracks[t].peak=0;tracks[t].sideLow[0]=tracks[t].sideLow[1]=0;tracks[t].haasPos=0;clear(haasL[t],sizeof(haasL[t]));clear(haasR[t],sizeof(haasR[t]));}for(int f=0;f<fxCount;f++){Effect* e=&effects[f];clear(arena+e->offset,e->size*2*sizeof(float));clear(e->state,sizeof(e->state));clear(e->b,sizeof(e->b));e->pos=0;e->phase=0;}}
/* GM percussion families must be explicit: an unknown note is not a pitched tom.
 * In particular Crash 2 (57), bongos (60/61) and congas (62-64) occur in the MIDI demos. */
static int is_cymbal(int pitch){return pitch==49||pitch==51||pitch==52||pitch==53||pitch==55||pitch==57||pitch==59;}
static float tom_hz(int pitch){
 switch(pitch){case 41:return 80;case 43:return 100;case 45:return 125;case 47:return 140;case 48:return 155;case 50:return 180;default:return 0;}
}
static float drum_length(int pitch,float decay){
 if(pitch==35||pitch==36)return .22f+decay*.35f;
 if(pitch==38||pitch==40)return .14f+decay;
 if(pitch==39)return .18f;
 if(pitch==42||pitch==44)return .065f;
 if(pitch==46)return .28f;
 if(is_cymbal(pitch))return 1.1f;
 if(tom_hz(pitch)>0)return .32f;
 switch(pitch){case 56:return .25f;case 60:return .20f;case 61:return .24f;case 62:return .12f;case 63:return .34f;case 64:return .38f;default:return .12f;}
}
__attribute__((export_name("note"))) void note(int track,int pitch,float velocity,int duration){
 if(track<0||track>=trackCount||pitch<0||pitch>127)return;
 int slot=-1;for(int i=0;i<VOICES;i++)if(!voices[i].live){slot=i;break;}
 if(slot<0){droppedCount++;return;}
 Voice* v=&voices[slot];clear(v,sizeof(*v));v->live=1;v->track=track;v->pitch=pitch;v->gate=duration;
 Track* t=&tracks[track];float* p=t->p;v->attack=(int)(clamp(p[6]*sr,1,duration*.45f));v->decay=(int)(p[7]*sr);v->release=(int)(p[9]*sr);v->amp=velocity;
 v->decayMul=decayFactor(p[7]*.3f);v->total=duration+v->release;
 if(p[0]==3){float length=drum_length(pitch,p[7]);v->total=(int)(length*sr);v->decayMul=decayFactor(length*.2f);v->env=1;}
 for(int bank=0;bank<2;bank++)for(int u=0;u<(int)p[bank?22:5];u++){
  int slot=bank*UNISON+u;
  float seed=p[bank?22:5]==1||p[32]==0?0:(noise()+1)*.5f;
  v->phase[slot]=wrap(p[31]+seed*p[32]);v->inc[slot]=clamp(frequencies[pitch]/sr*t->det[slot],.000001f,.45f);
 }
 v->subPhase=p[31];v->subInc=clamp(frequencies[pitch]/sr*exp2f_(p[29]),.000001f,.45f);
 activeCount++;if(activeCount>maxActive)maxActive=activeCount;
}
/* Manual audition note-off enters the same release envelope as scheduled notes. */
__attribute__((export_name("note_off"))) void note_off(int track,int pitch){
 for(int i=0;i<VOICES;i++){
  Voice* v=&voices[i];if(!v->live||v->track!=track||(pitch>=0&&v->pitch!=pitch)||v->age>=v->gate)continue;
  if(tracks[track].p[0]==3)continue; /* Percussion remains a natural one-shot. */
  v->gate=v->age;v->releaseStart=v->env;v->total=v->age+v->release;
 }
}
static float biquad(float x,float* c,BState* s){float y=c[0]*x+s->z1;s->z1=c[1]*x-c[3]*y+s->z2;s->z2=c[2]*x-c[4]*y;return y;}
/* Filter controls run on the note's sample clock, not the caller's block boundaries.
 * Interpolate RBJ coefficients over 16 samples to avoid stepped cutoff/LFO modulation. */
static void voice_coeff(Voice* v,float* p,int age,float* coeff){
 float cutoff=p[10]*exp2f_(p[16]*(1-clamp((float)age/(p[16]<0?v->gate:(v->decay?v->decay:1)),0,1))+p[18]*sinp((float)age/sr*p[17]));
 cutoff=clamp(cutoff,40,sr*.4f);
 float c=sinp(cutoff/sr+.25f),a=sinp(cutoff/sr)/(2*clamp(p[11],.1f,12)),inv=1/(1+a);
 coeff[0]=(1-c)*.5f*inv;coeff[1]=(1-c)*inv;coeff[2]=coeff[0];coeff[3]=-2*c*inv;coeff[4]=(1-a)*inv;
}
/* Three broad resonant air bands, not an unshaped white-noise bed.
 * Independent mid/side excitations have a per-voice PRNG: render chunk size and
 * other voices cannot change the stochastic texture. Width=0 collapses to mono.
 * Slow drift is continuous across blocks; coefficient ramps are sample-clocked. */
static float air_noise(Voice* v){v->airRandom^=v->airRandom<<13;v->airRandom^=v->airRandom>>17;v->airRandom^=v->airRandom<<5;return (float)(v->airRandom>>8)*(1.f/8388608.f)-1;}
static void air_coeff(Voice* v,float* p,float* c){
 float bright=clamp(p[14],0,1),drift=.15f*sinp(v->airMotion[0])+.08f*sinp(v->airMotion[1]);
 float centers[3]={650,1900,5100};float qs[3]={.62f,.8f,.65f};
 for(int band=0;band<3;band++){
  float hz=clamp(centers[band]*exp2f_((bright-.5f)*1.3f+drift),160,sr*.36f);
  float cosine=sinp(hz/sr+.25f),alpha=sinp(hz/sr)/(2*qs[band]),inv=1/(1+alpha);
  float* b=c+band*5;b[0]=alpha*inv;b[1]=0;b[2]=-b[0];b[3]=-2*cosine*inv;b[4]=(1-alpha)*inv;
 }
}
static void air_sample(Voice* v,float* p,float pitchFactor,float* left,float* right){
 if(!v->airRandom){
  noise();v->airRandom=randomState?randomState:1;
  v->airMotion[0]=(air_noise(v)+1)*.5f;v->airMotion[1]=(air_noise(v)+1)*.5f;
  v->airControl[0]=p[14];v->airControl[1]=p[3];v->airControl[2]=p[30];v->airControl[3]=p[15];
  air_coeff(v,p,v->airCoeff);
 }
 float targets[4]={p[14],p[3],p[30],p[15]};
 for(int j=0;j<4;j++)v->airControl[j]+=(targets[j]-v->airControl[j])*mixAlpha;
 if((v->age&63)==0){float next[15];air_coeff(v,p,next);for(int j=0;j<15;j++)v->airStep[j]=(next[j]-v->airCoeff[j])*(1.f/64);}
 float midNoise=air_noise(v),sideNoise=air_noise(v),mid=0,side=0,bright=v->airControl[0];
 float weights[3]={.5f-.25f*bright,.65f,.15f+.65f*bright};
 for(int band=0;band<3;band++){
  float breathe=.8f+.2f*sinp(v->airMotion[band&1]+band*.27f),weight=weights[band]*breathe;
  mid+=biquad(midNoise,v->airCoeff+band*5,&v->airFilter[band*2])*weight;
  side+=biquad(sideNoise,v->airCoeff+band*5,&v->airFilter[band*2+1])*weight;
 }
 float dt=clamp(v->inc[0]*pitchFactor*exp2f_(.0015f*sinp(v->airMotion[1])),.000001f,.45f);
 v->phase[0]=wrap(v->phase[0]+dt);
 /* Only integer harmonics, Nyquist-faded by partial(); no dissonant random pitches. */
 float tone=.7f*partial(v->phase[0],dt,1)+.2f*partial(v->phase[0],dt,2)+.1f*partial(v->phase[0],dt,4);
 float blend=v->airControl[1],width=clamp(v->airControl[3],0,1),level=v->airControl[2]*.65f*(1-blend);
 float norm=1/__builtin_sqrtf(1+width*width);
 *left=(mid+side*width)*norm*level+tone*blend*.16f;
 *right=(mid-side*width)*norm*level+tone*blend*.16f;
 for(int j=0;j<15;j++)v->airCoeff[j]+=v->airStep[j];
 v->airMotion[0]=wrap(v->airMotion[0]+.037f/sr);v->airMotion[1]=wrap(v->airMotion[1]+.061f/sr);
}
/* Coherent zero-detune stacks must not grow louder just because voice count increases. */
static float bank_gain(float* p,int bank){
 float count=p[bank?22:5],detune=p[bank?23:4];
 return .22f/(detune==0&&p[32]==0?count:__builtin_sqrtf(count));
}
static void fx_sample(Effect* e,float* l,float* r){
 float a=*l,b=*r,dl=a,dr=b;float* p=e->p;float* d=arena+e->offset;
 if(e->type==0){for(int j=0;j<3;j++){dl=biquad(dl,p+j*5,&e->b[j*2]);dr=biquad(dr,p+j*5,&e->b[j*2+1]);}}
 else if(e->type==1){dl=biquad(a,p,&e->b[0]);dr=biquad(b,p,&e->b[1]);}
 else if(e->type==2){float g=p[0];dl=sat(a*g)/__builtin_sqrtf(g);dr=sat(b*g)/__builtin_sqrtf(g);}
 else if(e->type==3){ // Opposite-phase interpolated delay chorus, never creates oscillators.
  int pos=e->pos;d[pos]=a;d[4096+pos]=b;
  for(int c=0;c<2;c++){float delay=sr*(.012f+p[1]*sinp(e->phase+c*.5f));float at=pos-delay;if(at<0)at+=4096;int k=(int)at;float f=at-k;float value=d[c*4096+k]*(1-f)+d[c*4096+((k+1)&4095)]*f;if(c==0)dl=value;else dr=value;}
  e->pos=(pos+1)&4095;e->phase=wrap(e->phase+p[0]/sr);
 }else if(e->type==4){int pos=e->pos;int lag=(int)p[0];if(lag>=e->size)lag=e->size-1;int at=pos-lag;if(at<0)at+=e->size;
  dl=d[at];dr=d[e->size+at];float fb=p[1];d[pos]=a+(p[3]>.5f?dr:dl)*fb;d[e->size+pos]=b+(p[3]>.5f?dl:dr)*fb;e->pos=pos+1==e->size?0:pos+1;
 }else if(e->type==5){ // Four-line damped, orthogonal feedback delay network, stereo output.
  int pos=e->pos;float v[4];int lengths[4]={1499,1789,2131,2539};
  for(int j=0;j<4;j++){int len=(int)(lengths[j]*p[0]);len= (int)clamp(len,80,4095);int at=(pos-len)&4095;float x=d[j*4096+at];e->state[j]+=.32f*(x-e->state[j]);v[j]=e->state[j];}
  float sum=(v[0]+v[1]+v[2]+v[3])*.5f;
  for(int j=0;j<4;j++)d[j*4096+pos]=((j&1)?b:a)*.4f+(sum-v[j])*p[j+1];
  dl=(v[0]+v[2]-v[1])*.65f;dr=(v[1]+v[3]-v[2])*.65f;e->pos=(pos+1)&4095;
 }else if(e->type==6){float peak=absf(a)>absf(b)?absf(a):absf(b);float coef=peak>e->state[0]?p[2]:p[3];e->state[0]=peak+(e->state[0]-peak)*coef;float ratio=e->state[0]/p[0];float gain=ratio>1?1/(1+(ratio-1)*(1-1/p[1])):1;dl=a*gain;dr=b*gain;}
 *l=a*(1-e->mix)+dl*e->mix;*r=b*(1-e->mix)+dr*e->mix;
}
__attribute__((export_name("render"))) void render(int frames){
 if(frames<1||frames>128)return;
 for(int t=0;t<trackCount;t++){tracks[t].peak=0;for(int i=0;i<frames;i++)busL[t][i]=busR[t][i]=0;}
 for(int k=0;k<VOICES;k++){
  Voice* v=&voices[k];if(!v->live)continue;Track* tr=&tracks[v->track];float* p=tr->p;int engine=(int)p[0];
  float norms[2]={bank_gain(p,0),bank_gain(p,1)};
  for(int i=0;i<frames;i++){
   if(v->age>=v->total){v->live=0;activeCount--;break;}float l=0,r=0;
   float pitchFactor=p[19]==0?1:exp2f_(p[19]/12*clamp((float)v->age/(v->gate?v->gate:1),0,1));
   if(engine!=3 && (v->age&15)==0){
    if(v->age==0)voice_coeff(v,p,0,v->coeff);
    float next[5];voice_coeff(v,p,v->age+16,next);
    for(int j=0;j<5;j++)v->coeffStep[j]=(next[j]-v->coeff[j])*(1.f/16);
   }
   if(engine==3){
    float t=(float)v->age/sr;int pitch=v->pitch;float n=noise();v->noiseLow+=.12f*(n-v->noiseLow);float high=(n-v->noiseLow)*(.25f+.75f*p[14]);
    if(pitch==35||pitch==36){float hz=47+125*exp2f_(-t*70);v->drumPhase=wrap(v->drumPhase+hz/sr);l=sat(sinp(v->drumPhase)*1.8f)*v->env*.72f+high*.1f*exp2f_(-t*280);}
    else if(pitch==38||pitch==40){v->drumPhase=wrap(v->drumPhase+(175+80*exp2f_(-t*50))/sr);l=(sinp(v->drumPhase)*exp2f_(-t*35)*.55f+high*.65f)*v->env;}
    else if(pitch==39){float burst=t<.024f?(.3f+.7f*sinp(t*130)*sinp(t*130)):1;l=high*.7f*v->env*burst;}
    else if(pitch==42||pitch==44||pitch==46||is_cymbal(pitch)){l=high*v->env*.25f;}
    else if(tom_hz(pitch)>0){float hz=tom_hz(pitch);v->drumPhase=wrap(v->drumPhase+hz/sr*(1+.5f*exp2f_(-t*30)));l=sinp(v->drumPhase)*v->env*.5f+high*.03f*v->env;}
    else if(pitch>=60&&pitch<=64){
     /* Hand drums: fixed membrane modes plus a brief slap, without a synth pitch dive. */
     float hz=pitch==60?550:pitch==61?370:pitch==62?320:pitch==63?285:210;
     v->drumPhase=wrap(v->drumPhase+hz/sr);v->phase[0]=wrap(v->phase[0]+hz*1.59f/sr);v->phase[1]=wrap(v->phase[1]+hz*2.14f/sr);
     l=(sinp(v->drumPhase)*.42f+sinp(v->phase[0])*.20f*exp2f_(-t*45)+sinp(v->phase[1])*.10f*exp2f_(-t*70)+high*.12f*exp2f_(-t*90))*v->env;
    }
    else if(pitch==56){v->drumPhase=wrap(v->drumPhase+540/sr);v->phase[0]=wrap(v->phase[0]+845/sr);l=(sinp(v->drumPhase)*.32f+sinp(v->phase[0])*.23f)*v->env;}
    else{l=high*v->env*.32f;} /* Neutral short noise for unsupported percussion. */
    /* A short end taper avoids a discontinuity when the one-shot voice expires. */
    l*=clamp((v->total-v->age)/(sr*.005f),0,1);
    v->env*=v->decayMul;l*=v->amp;r=l;
   }else{
    if(v->age<v->gate){if(v->age<v->attack){float u=(float)v->age/(v->attack?v->attack:1);v->env=u*u*(3-2*u);}else if(v->age==v->attack)v->env=1;else v->env=p[8]+(v->env-p[8])*v->decayMul;v->releaseStart=v->env;}
    else {float remaining=clamp(1.f-(float)(v->age-v->gate)/(v->release?v->release:1),0,1);v->env=v->releaseStart*remaining*remaining;}
    if(engine==4)air_sample(v,p,pitchFactor,&l,&r);
    else if(engine==1){float index=p[13]*(.12f+.88f*exp2f_(-(float)v->age/(sr*p[7])*4));v->fm=wrap(v->fm+v->inc[0]*p[12]*pitchFactor);v->phase[0]=wrap(v->phase[0]+v->inc[0]*pitchFactor);l=r=sinp(v->phase[0]+sinp(v->fm)*index*.15915494f)*.28f;}
    else for(int bank=0;bank<2;bank++){
     float gain=norms[bank]*(bank?p[3]:1-p[3]);
     if(gain==0)continue;
     for(int u=0;u<(int)p[bank?22:5];u++){int slot=bank*UNISON+u;float dt=clamp(v->inc[slot]*pitchFactor,.000001f,.45f);
      v->phase[slot]=wrap(v->phase[slot]+dt);float x=osc((int)p[bank?2:1],v->phase[slot],dt)*gain;
      l+=x*tr->pl[slot];r+=x*tr->pr[slot];
     }
    }
    if((engine==0||engine==1||engine==2)&&p[30]>0){float n=noise()*.22f*p[30];l+=n;r+=n;}
    l=biquad(l,v->coeff,&v->filter[0])*v->env*v->amp;r=biquad(r,v->coeff,&v->filter[1])*v->env*v->amp;
    for(int j=0;j<5;j++)v->coeff[j]+=v->coeffStep[j];
    if((engine==0||engine==1||engine==2)&&p[28]>0){v->subPhase=wrap(v->subPhase+clamp(v->subInc*pitchFactor,.000001f,.45f));float sub=sinp(v->subPhase)*p[28]*.22f*v->env*v->amp;l+=sub;r+=sub;}
   }
   busL[v->track][i]+=l;busR[v->track][i]+=r;v->age++;
  }
 }
 for(int i=0;i<frames;i++)output[i]=output[128+i]=0;
 for(int t=0;t<trackCount;t++){
  Track* tr=&tracks[t];float dl=(tr->level-tr->smLevel)/frames,dp=(tr->pan-tr->smPan)/frames,dc=(tr->cutoff-tr->smCutoff)/frames;
  for(int i=0;i<frames;i++){
   tr->smLevel+=dl;tr->smPan+=dp;tr->smCutoff+=dc;float l=busL[t][i],r=busR[t][i];
   float alpha=clamp(tr->smCutoff/sr*4,0,1);tr->lp[0]+=alpha*(l-tr->lp[0]);tr->lp[1]+=alpha*(r-tr->lp[1]);l=tr->lp[0];r=tr->lp[1];
   for(int f=tr->first;f>=0;f=effects[f].next)fx_sample(&effects[f],&l,&r);
   // Fractional, feedback-free Haas delay on exactly one side. Buffers also run
   // on silence so the delayed tail survives a note-off; reset/seek clears them.
   if(tr->p[33]!=0&&tr->p[34]>0){
    int pos=tr->haasPos;haasL[t][pos]=l;haasR[t][pos]=r;
    float lag=clamp(absf(tr->p[33])*sr*.001f,0,HAAS_SIZE-2);int whole=(int)lag;float frac=lag-whole;
    int at=(pos-whole)&(HAAS_SIZE-1),prev=(at-1)&(HAAS_SIZE-1);float mix=tr->p[34];
    if(tr->p[33]>0)r=r*(1-mix)+(haasR[t][at]*(1-frac)+haasR[t][prev]*frac)*mix;
    else l=l*(1-mix)+(haasL[t][at]*(1-frac)+haasL[t][prev]*frac)*mix;
    tr->haasPos=(pos+1)&(HAAS_SIZE-1);
   }
   // Complementary M/S crossover: retain the mid, attenuate low-frequency side.
   // After inserts and Haas, so neither can spread the sub again.
   if(tr->p[35]>0){float mid=(l+r)*.5f,side=(l-r)*.5f;for(int pole=0;pole<2;pole++){tr->sideLow[pole]+=tr->sideAlpha*(side-tr->sideLow[pole]);side-=tr->sideLow[pole];}l=mid+side;r=mid-side;}
   float pan=tr->smPan;if(pan>0){r+=l*pan;l*=1-pan;}else{l-=r*pan;r*=1+pan;}
   tr->smGain+=(tr->p[20]-tr->smGain)*mixAlpha;
   if(absf(tr->p[20]-tr->smGain)<.0000001f)tr->smGain=tr->p[20];
   l*=tr->smLevel*tr->smGain;r*=tr->smLevel*tr->smGain;float peak=absf(l)>absf(r)?absf(l):absf(r);if(peak>tr->peak)tr->peak=peak;output[i]+=l;output[128+i]+=r;
  }
 }
 for(int i=0;i<frames;i++){smMaster+=(master-smMaster)*mixAlpha;output[i]=sat(output[i]*smMaster)*.92f;output[128+i]=sat(output[128+i]*smMaster)*.92f;}
}

"""100x100 Python math workload; checksum must remain 1267865."""
import math, sys, time
sources=[(0.28,0.30,1.00),(0.74,0.68,0.85),(0.52,0.12,0.55)]
k=30.0*math.pi
wsum=sum(w for _,_,w in sources)
start=time.perf_counter()
checksum=0
for y in range(100):
    v=y/99
    for x in range(100):
        u=x/99
        amp=0.0
        for sx,sy,w in sources:
            dx=u-sx
            dy=v-sy
            d=math.sqrt(dx*dx+dy*dy)
            amp+=w*math.sin(k*d)/(1.0+3.2*d)
        t=max(0.0,min(1.0,0.5+1.24*amp/wsum))
        glow=t*t*(3-2*t)
        checksum+=int(255*glow)
print('loop_seconds=%.6f checksum=%d' % (time.perf_counter()-start, checksum))

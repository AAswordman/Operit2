#include "mini_font.h"
#include <stdio.h>
#include <stdlib.h>
static void export(const char *name,const mini_glyph_t *g,size_t count,const uint8_t *bitmap) {
 FILE *f=fopen(name,"wb");if(!f)exit(2);
 fprintf(f,"STARTFONT 2.1\nFONT -Operit-NotoSansCJKSC-Regular-R-Normal--14-140-72-72-P-140-ISO10646-1\nSIZE 14 72 72\nFONTBOUNDINGBOX 32 40 -10 -10\nSTARTPROPERTIES 2\nFONT_ASCENT 30\nFONT_DESCENT 10\nENDPROPERTIES\nCHARS %zu\n",count);
 for(size_t i=0;i<count;i++){
  const mini_glyph_t *p=g+i;const unsigned char *bits=bitmap+p->offset;
  fprintf(f,"STARTCHAR uni%04X\nENCODING %u\nSWIDTH %u 0\nDWIDTH %u 0\nBBX %u %u %d %d\nBITMAP\n",p->codepoint,p->codepoint,p->advance*1000/14,p->advance,p->w,p->h,p->x,p->y);
  for(unsigned row=0;row<p->h;row++){
   unsigned value=0,used=0;
   for(unsigned col=0;col<p->w;col++){
    unsigned bit=(row*p->w+col)*p->bpp,v=(bits[bit/8]>>(8-p->bpp-bit%8))&((1u<<p->bpp)-1);
    value=(value<<1)|(v!=0);if(++used==8){fprintf(f,"%02X",value);value=0;used=0;}
   }
   if(used)fprintf(f,"%02X",value<<(8-used));fputc('\n',f);
  }
  fprintf(f,"ENDCHAR\n");
 }
 fprintf(f,"ENDFONT\n");fclose(f);
}
int main(int argc,char **argv){if(argc!=3)return 1;
 export(argv[1],mini_text_glyphs,sizeof(mini_text_glyphs)/sizeof(*mini_text_glyphs),mini_text_bitmap);
 export(argv[2],mini_digits_glyphs,sizeof(mini_digits_glyphs)/sizeof(*mini_digits_glyphs),mini_digits_bitmap);
 for(unsigned i=0;i<sizeof(mini_text_glyphs)/sizeof(*mini_text_glyphs);i++) {
  const mini_glyph_t *g=mini_text_glyphs+i;
  printf("%u %u %u %d %d %u %u %u %u\n",g->codepoint,g->w,g->h,g->x,g->y,g->advance,g->line,g->baseline,g->bpp);
 }
 return 0;
}

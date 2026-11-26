#include "mini_font.h"
#include "u8g2.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "text-u8g2.c"
#include "digits-u8g2.c"
void u8g2_read_font_info(u8g2_font_info_t *,const uint8_t *);
const uint8_t *u8g2_font_get_glyph_data(u8g2_t *,uint16_t);
int8_t u8g2_font_decode_glyph(u8g2_t *,const uint8_t *);
static unsigned char actual[64*64],expected[64*64];
void u8g2_DrawHVLine(u8g2_t *d,u8g2_uint_t x,u8g2_uint_t y,u8g2_uint_t n,uint8_t dir){
 if(dir!=0)abort();
 for(unsigned i=0;i<n;i++){if(x+i>=64||y>=64)abort();actual[y*64+x+i]=d->draw_color;}
}
static unsigned check(const mini_glyph_t *g,size_t count,const uint8_t *bitmap,const uint8_t *font){
 unsigned errors=0;
 for(size_t k=0;k<count;k++){
  const mini_glyph_t *a=g+k;memset(actual,0,sizeof(actual));memset(expected,0,sizeof(expected));
  u8g2_t d={0};d.font=font;d.draw_color=1;d.font_decode.is_transparent=1;
  u8g2_read_font_info(&d.font_info,font);
  const uint8_t *glyph=u8g2_font_get_glyph_data(&d,a->codepoint);
  if(!glyph){printf("missing U+%04X\n",a->codepoint);errors++;continue;}
  d.font_decode.target_x=16-a->x;d.font_decode.target_y=16+a->h+a->y;
  int8_t advance=u8g2_font_decode_glyph(&d,glyph);
  const uint8_t *bits=bitmap+a->offset;
  for(unsigned row=0;row<a->h;row++)for(unsigned col=0;col<a->w;col++){
   unsigned bit=(row*a->w+col)*a->bpp,v=(bits[bit/8]>>(8-a->bpp-bit%8))&((1u<<a->bpp)-1);
   expected[(16+row)*64+16+col]=v!=0;
  }
  if(advance!=a->advance||memcmp(actual,expected,sizeof(actual))){
   if(errors<10)printf("mismatch U+%04X advance %d/%u\n",a->codepoint,advance,a->advance);errors++;
  }
 }
 return errors;
}
int main(void){
 unsigned count=sizeof(mini_text_glyphs)/sizeof(*mini_text_glyphs)+sizeof(mini_digits_glyphs)/sizeof(*mini_digits_glyphs);
 unsigned errors=check(mini_text_glyphs,sizeof(mini_text_glyphs)/sizeof(*mini_text_glyphs),mini_text_bitmap,operit_font_text)
  +check(mini_digits_glyphs,sizeof(mini_digits_glyphs)/sizeof(*mini_digits_glyphs),mini_digits_bitmap,operit_font_digits);
 printf("{\"glyphs\":%u,\"mismatches\":%u,\"originalDataBytes\":%zu,\"u8g2FontBytes\":%zu}\n",count,errors,
 sizeof(mini_text_glyphs)+sizeof(mini_digits_glyphs)+sizeof(mini_text_bitmap)+sizeof(mini_digits_bitmap),sizeof(operit_font_text)+sizeof(operit_font_digits));
 return errors!=0;
}

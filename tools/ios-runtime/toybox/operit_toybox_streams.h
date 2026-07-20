#ifndef OPERIT_TOYBOX_STREAMS_H
#define OPERIT_TOYBOX_STREAMS_H

extern FILE *operit_toybox_stdin;
extern FILE *operit_toybox_stdout;
extern FILE *operit_toybox_stderr;

#undef stdin
#undef stdout
#undef stderr
#define stdin operit_toybox_stdin
#define stdout operit_toybox_stdout
#define stderr operit_toybox_stderr

#endif

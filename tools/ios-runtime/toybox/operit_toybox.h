#ifndef OPERIT_TOYBOX_H
#define OPERIT_TOYBOX_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OperitToyboxResult {
  int exit_code;
  char *standard_output;
  char *standard_error;
} OperitToyboxResult;

/// Runs one enabled Toybox applet against the supplied workspace directory.
int operit_toybox_run(
    const char *working_directory,
    int argc,
    const char *const argv[],
    const char *standard_input,
    OperitToyboxResult *result);

/// Reports whether a named applet is compiled into the embedded Toybox runtime.
int operit_toybox_has_applet(const char *name);

/// Releases all caller-owned output strings returned by the Toybox runtime.
void operit_toybox_result_free(OperitToyboxResult *result);

#ifdef __cplusplus
}
#endif

#endif

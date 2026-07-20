#include "toys.h"
#include "operit_toybox.h"

FILE *operit_toybox_stdin;
FILE *operit_toybox_stdout;
FILE *operit_toybox_stderr;

/// Stores one static failure description in the public Toybox result value.
static int operit_toybox_fail(OperitToyboxResult *result, int status, const char *message) {
  result->exit_code = 1;
  result->standard_output = strdup("");
  result->standard_error = strdup(message);
  return status;
}

/// Rejects applets whose Toybox metadata requires privileged process state.
static int operit_toybox_applet_is_supported(const struct toy_list *applet) {
  const unsigned unsupported = TOYFLAG_ROOTONLY;
  return applet != 0 && !(applet->flags & unsupported);
}

/// Duplicates the caller argument vector because Toybox option handling can modify it.
static char **operit_toybox_duplicate_arguments(int argc, const char *const argv[]) {
  char **arguments = calloc((size_t)argc + 1, sizeof(char *));
  int index;

  if (!arguments) return 0;
  for (index = 0; index < argc; index++) {
    arguments[index] = strdup(argv[index]);
    if (!arguments[index]) {
      while (index--) free(arguments[index]);
      free(arguments);
      return 0;
    }
  }
  return arguments;
}

/// Frees a duplicated Toybox argument vector.
static void operit_toybox_free_arguments(char **arguments) {
  int index;

  if (!arguments) return;
  for (index = 0; arguments[index]; index++) free(arguments[index]);
  free(arguments);
}

/// Reads one complete temporary stream into caller-owned NUL-terminated storage.
static int operit_toybox_read_stream(FILE *stream, char **value) {
  long length;
  char *contents;

  if (fflush(stream) || fseek(stream, 0, SEEK_END)) return -1;
  length = ftell(stream);
  if (length < 0 || fseek(stream, 0, SEEK_SET)) return -1;
  contents = malloc((size_t)length + 1);
  if (!contents) return -1;
  if ((long)fread(contents, 1, (size_t)length, stream) != length) {
    free(contents);
    return -1;
  }
  contents[length] = 0;
  *value = contents;
  return 0;
}

/// Restores the standard descriptors after a serialized Toybox applet invocation.
static int operit_toybox_restore_descriptors(int input, int output, int error) {
  int status = 0;

  if (dup2(input, STDIN_FILENO) < 0) status = errno;
  if (dup2(output, STDOUT_FILENO) < 0 && !status) status = errno;
  if (dup2(error, STDERR_FILENO) < 0 && !status) status = errno;
  close(input);
  close(output);
  close(error);
  return status;
}

/// Reports whether a named applet exists in the compiled Toybox registry.
int operit_toybox_has_applet(const char *name) {
  return name && toy_find((char *)name) != 0;
}

/// Releases all dynamically allocated fields in a Toybox execution result.
void operit_toybox_result_free(OperitToyboxResult *result) {
  if (!result) return;
  free(result->standard_output);
  free(result->standard_error);
  result->standard_output = 0;
  result->standard_error = 0;
  result->exit_code = 0;
}

/// Runs one applet with dedicated memory streams and restores the prior process directory.
int operit_toybox_run(
    const char *working_directory,
    int argc,
    const char *const argv[],
    const char *standard_input,
    OperitToyboxResult *result) {
  char *input_copy;
  char **arguments;
  char *standard_output = 0;
  char *standard_error = 0;
  FILE *input_stream;
  FILE *output_stream;
  FILE *error_stream;
  struct toy_list *applet;
  int original_directory;
  int saved_input;
  int saved_output;
  int saved_error;
  int descriptor_status;
  int exited;

  if (!result) return EINVAL;
  result->exit_code = 1;
  result->standard_output = 0;
  result->standard_error = 0;
  if (!working_directory || argc < 1 || !argv || !argv[0])
    return operit_toybox_fail(result, EINVAL, "Toybox requires a command and workspace directory");
  applet = toy_find((char *)argv[0]);
  if (!applet) return operit_toybox_fail(result, ENOENT, "Toybox applet is not compiled into this runtime");
  if (!operit_toybox_applet_is_supported(applet))
    return operit_toybox_fail(result, ENOTSUP, "Toybox applet requires unsupported process behavior");
  arguments = operit_toybox_duplicate_arguments(argc, argv);
  if (!arguments) return operit_toybox_fail(result, ENOMEM, "Toybox cannot allocate command arguments");
  if (!standard_input) {
    operit_toybox_free_arguments(arguments);
    return operit_toybox_fail(result, EINVAL, "Toybox requires standard input text");
  }
  input_copy = strdup(standard_input);
  if (!input_copy) {
    operit_toybox_free_arguments(arguments);
    return operit_toybox_fail(result, ENOMEM, "Toybox cannot allocate standard input");
  }
  input_stream = tmpfile();
  output_stream = tmpfile();
  error_stream = tmpfile();
  if (!input_stream || !output_stream || !error_stream) {
    if (input_stream) fclose(input_stream);
    if (output_stream) fclose(output_stream);
    if (error_stream) fclose(error_stream);
    free(input_copy);
    operit_toybox_free_arguments(arguments);
    free(standard_output);
    free(standard_error);
    return operit_toybox_fail(result, errno, "Toybox cannot create command streams");
  }
  if (fwrite(input_copy, 1, strlen(input_copy), input_stream) != strlen(input_copy)
      || fseek(input_stream, 0, SEEK_SET)) {
    fclose(input_stream);
    fclose(output_stream);
    fclose(error_stream);
    free(input_copy);
    operit_toybox_free_arguments(arguments);
    return operit_toybox_fail(result, errno, "Toybox cannot initialize standard input");
  }
  original_directory = open(".", O_RDONLY);
  if (original_directory < 0 || chdir(working_directory)) {
    if (original_directory >= 0) close(original_directory);
    fclose(input_stream);
    fclose(output_stream);
    fclose(error_stream);
    free(input_copy);
    operit_toybox_free_arguments(arguments);
    free(standard_output);
    free(standard_error);
    return operit_toybox_fail(result, errno, "Toybox cannot enter the workspace directory");
  }
  saved_input = dup(STDIN_FILENO);
  saved_output = dup(STDOUT_FILENO);
  saved_error = dup(STDERR_FILENO);
  if (saved_input < 0 || saved_output < 0 || saved_error < 0) {
    descriptor_status = errno;

    if (saved_input >= 0) close(saved_input);
    if (saved_output >= 0) close(saved_output);
    if (saved_error >= 0) close(saved_error);
    fchdir(original_directory);
    close(original_directory);
    fclose(input_stream);
    fclose(output_stream);
    fclose(error_stream);
    free(input_copy);
    operit_toybox_free_arguments(arguments);
    return operit_toybox_fail(result, descriptor_status, "Toybox cannot save standard descriptors");
  }
  fflush(0);
  if (dup2(fileno(input_stream), STDIN_FILENO) < 0
      || dup2(fileno(output_stream), STDOUT_FILENO) < 0
      || dup2(fileno(error_stream), STDERR_FILENO) < 0) {
    descriptor_status = errno;

    operit_toybox_restore_descriptors(saved_input, saved_output, saved_error);
    fchdir(original_directory);
    close(original_directory);
    fclose(input_stream);
    fclose(output_stream);
    fclose(error_stream);
    free(input_copy);
    operit_toybox_free_arguments(arguments);
    return operit_toybox_fail(result, descriptor_status, "Toybox cannot route standard descriptors");
  }
  operit_toybox_stdin = input_stream;
  operit_toybox_stdout = output_stream;
  operit_toybox_stderr = error_stream;
  NOEXIT({
    toy_init(applet, arguments);
    applet->toy_main();
    xexit();
  });
  exited = toys.exitval;
  fflush(0);
  descriptor_status = operit_toybox_restore_descriptors(saved_input, saved_output, saved_error);
  if (fchdir(original_directory)) {
    int restore_status = errno;

    close(original_directory);
    fclose(input_stream);
    fclose(output_stream);
    fclose(error_stream);
    operit_toybox_stdin = 0;
    operit_toybox_stdout = 0;
    operit_toybox_stderr = 0;
    free(input_copy);
    operit_toybox_free_arguments(arguments);
    return operit_toybox_fail(result, restore_status, "Toybox cannot restore the process directory");
  }
  close(original_directory);
  operit_toybox_stdin = 0;
  operit_toybox_stdout = 0;
  operit_toybox_stderr = 0;
  if (descriptor_status) {
    fclose(input_stream);
    fclose(output_stream);
    fclose(error_stream);
    free(input_copy);
    operit_toybox_free_arguments(arguments);
    return operit_toybox_fail(result, descriptor_status, "Toybox cannot restore standard descriptors");
  }
  if (operit_toybox_read_stream(output_stream, &standard_output)
      || operit_toybox_read_stream(error_stream, &standard_error)) {
    fclose(input_stream);
    fclose(output_stream);
    fclose(error_stream);
    free(input_copy);
    operit_toybox_free_arguments(arguments);
    free(standard_output);
    free(standard_error);
    return operit_toybox_fail(result, errno, "Toybox cannot capture command output");
  }
  fclose(input_stream);
  fclose(output_stream);
  fclose(error_stream);
  free(input_copy);
  operit_toybox_free_arguments(arguments);
  result->exit_code = exited;
  result->standard_output = standard_output;
  result->standard_error = standard_error;
  if (!result->standard_output || !result->standard_error) {
    operit_toybox_result_free(result);
    return operit_toybox_fail(result, ENOMEM, "Toybox cannot retain command output");
  }
  return 0;
}

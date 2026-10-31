#pragma once

#include <flutter_linux/flutter_linux.h>
#include <string>

struct WindowConfiguration {
  std::string arguments;
  bool hidden_at_launch = false;
  bool transparent = false;

  static WindowConfiguration FromFlValue(FlValue* value) {
    WindowConfiguration config;

    if (!value || fl_value_get_type(value) != FL_VALUE_TYPE_MAP) {
      return config;
    }

    FlValue* arguments_value = fl_value_lookup_string(value, "arguments");
    if (arguments_value &&
        fl_value_get_type(arguments_value) == FL_VALUE_TYPE_STRING) {
      config.arguments = fl_value_get_string(arguments_value);
    }

    FlValue* hidden_value = fl_value_lookup_string(value, "hiddenAtLaunch");
    if (hidden_value && fl_value_get_type(hidden_value) == FL_VALUE_TYPE_BOOL) {
      config.hidden_at_launch = fl_value_get_bool(hidden_value);
    }

    FlValue* transparent_value = fl_value_lookup_string(value, "transparent");
    if (transparent_value && fl_value_get_type(transparent_value) == FL_VALUE_TYPE_BOOL) {
      config.transparent = fl_value_get_bool(transparent_value);
    }
    return config;
  }

  FlValue* ToFlValue() const {
    g_autoptr(FlValue) result = fl_value_new_map();

    fl_value_set_string_take(result, "arguments",
                             fl_value_new_string(arguments.c_str()));

    fl_value_set_string_take(result, "hiddenAtLaunch",
                             fl_value_new_bool(hidden_at_launch));

    fl_value_set_string_take(result, "transparent", fl_value_new_bool(transparent));
    return fl_value_ref(result);
  }
};
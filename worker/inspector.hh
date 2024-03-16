#pragma once

#include <nix/config.h>

#include <memory>
#include <nix/primops.hh>

#include "command.hh"
#include "eval.hh"
#include "nixexpr.hh"
#include "types.hh"
#include "value.hh"

using Value = nix::Value;
struct NixInspector;

using namespace nix;

// using ValueType = nix::ValueType;

// struct NixAttr {
//   rust::String key;
//   std::unique_ptr<Value> value;
// };

// Capture all logs to an internal stream
// TODO: expose this stream of data in the UI
class CaptureLogger : public Logger {
  std::ostringstream oss;

 public:
  CaptureLogger() {}

  std::string get() const { return oss.str(); }

  void log(Verbosity lvl, std::string_view s) override {
    oss << s << std::endl;
  }

  void logEI(const ErrorInfo &ei) override {
    showErrorInfo(oss, ei, loggerSettings.showTrace.get());
  }
};

// nix is designed with command-line use in mind, and there's some setup stuff
// that's tied to the EvalCommand class.
struct NixInspector : virtual EvalCommand {
 public:
  EvalState *state;
  Value &vRoot;
  Bindings &autoArgs;

  NixInspector();
  void addAttrsToScope(Value &attrs);
  ref<Store> getEvalStore();

  std::shared_ptr<Value> inspect(const std::string &attrPaths);
  ValueType v_type(const Value &value);
  int32_t v_int(const Value &value);
  float_t v_float(const Value &value);
  bool v_bool(const Value &value);
  std::string v_string(const Value &value);
  std::string v_path(const Value &value);
  // std::vector<NixAttr> v_attrs(const Value &value);
  std::unique_ptr<std::vector<Value>> v_list(const Value &value);
  std::shared_ptr<Value> v_child(const Value &value, std::string key);
  nlohmann::json v_repr(const Value &value);

  void run(ref<Store> store) override {
    // so it doesn't complain about unused variables
    (void)store;
  }
};

void init_nix_inspector();
std::unique_ptr<NixInspector> new_nix_inspector();

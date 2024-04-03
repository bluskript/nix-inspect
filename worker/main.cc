#include <iostream>
#include <nlohmann/json.hpp>
#include <nlohmann/json_fwd.hpp>
#include <ostream>

#include "inspector.hh"

volatile sig_atomic_t stop;

void inthand(int signum) {
  stop = 1;
  throw std::runtime_error("Interrupted by Ctrl+C");
}

int main() {
  signal(SIGINT, inthand);
  init_nix_inspector();
  auto inspector = NixInspector();
  while (!stop) {
    std::string data;
    std::cin >> data;
    try {
      auto value = inspector.inspect(data);
      nlohmann::json out = {
          {"type", std::to_string(value->type())},
          {"data", inspector.v_repr(*value)}
      };
      std::cout << out << std::endl;
    } catch (...) {
      std::cout << "error" << std::endl;
    }
  }
  return 0;
}

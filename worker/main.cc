#include <iostream>
#include <nlohmann/json.hpp>
#include <nlohmann/json_fwd.hpp>
#include <ostream>

#include "inspector.hh"

int main() {
  init_nix_inspector();
  std::string expr;
  getline(std::cin, expr);
  auto inspector = NixInspector(expr);
  std::string data;
  while (getline(std::cin, data)) {
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

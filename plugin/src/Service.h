#pragma once
#include "Bank.h"
namespace loops {
std::filesystem::path profileRoot();
std::filesystem::path resourceRoot();
class Service {
  std::string instance;
  Json exchange(const Json &request, bool launch);

public:
  Service();
  Json request(const std::string &action, const Json &data = Json::object());
};
} // namespace loops

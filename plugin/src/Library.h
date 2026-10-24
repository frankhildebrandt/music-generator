#pragma once
#include "Bank.h"
namespace loops {
Json factoryTemplates();
std::string fullPrompt(const Json &document,
                       const std::string &instrumentPrompt);
std::string noteName(int midi);
} // namespace loops

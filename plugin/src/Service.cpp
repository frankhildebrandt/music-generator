#include "Service.h"
#include <chrono>
#include <fstream>
#include <iomanip>
#include <random>
#include <sstream>
#include <stdexcept>
#include <thread>
#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#include <spawn.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>
extern char **environ;
#endif
namespace loops {
std::filesystem::path profileRoot() {
#ifdef _WIN32
  std::array<wchar_t, 32768> value{};
  if (GetEnvironmentVariableW(L"MUSIC_LOOPS_PROFILE", value.data(),
                              DWORD(value.size())))
    return value.data();
  if (!GetEnvironmentVariableW(L"LOCALAPPDATA", value.data(),
                               DWORD(value.size())))
    throw std::runtime_error("Missing LOCALAPPDATA");
  return std::filesystem::path(value.data()) / "MusicGeneratorLoops";
#else
  if (auto *env = std::getenv("MUSIC_LOOPS_PROFILE"))
    return env;
  return std::filesystem::path(std::getenv("HOME")) /
         "Library/Application Support/MusicGeneratorLoops";
#endif
}
std::filesystem::path resourceRoot() {
#ifdef _WIN32
  HMODULE module = nullptr;
  GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
                         GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                     reinterpret_cast<LPCWSTR>(&resourceRoot), &module);
  std::array<wchar_t, 32768> path{};
  GetModuleFileNameW(module, path.data(), DWORD(path.size()));
  return std::filesystem::path(path.data()).parent_path() /
         "MusicGeneratorLoops.resources";
#else
  Dl_info info{};
  dladdr(reinterpret_cast<void *>(&resourceRoot), &info);
  return std::filesystem::path(info.dli_fname).parent_path().parent_path() /
         "Resources";
#endif
}
Service::Service() {
  std::random_device r;
  instance = std::to_string(r()) + "-" + std::to_string(r());
}
Json Service::request(const std::string &action, const Json &data) {
  return exchange({{"version", 1},
                   {"instance", instance},
                   {"action", action},
                   {"data", data}},
                  action != "detach");
}
Json Service::exchange(const Json &request, bool launch) {
  const auto text = request.dump();
  if (text.size() > 1024 * 1024)
    throw std::runtime_error("Service request too large");
#ifdef _WIN32
  uint64_t hash = 14695981039346656037ULL;
  for (wchar_t unit : profileRoot().wstring()) {
    hash ^= uint16_t(unit);
    hash *= 1099511628211ULL;
  }
  std::wostringstream name;
  name << L"\\\\.\\pipe\\MusicGeneratorLoops-v1-" << std::hex << std::setw(16)
       << std::setfill(L'0') << hash;
  const std::wstring pipe = name.str();
  HANDLE fd = INVALID_HANDLE_VALUE;
  for (int attempt = 0; attempt < 100; ++attempt) {
    fd = CreateFileW(pipe.c_str(), GENERIC_READ | GENERIC_WRITE, 0, nullptr,
                     OPEN_EXISTING, 0, nullptr);
    if (fd != INVALID_HANDLE_VALUE)
      break;
    if (!launch)
      throw std::runtime_error("Service unavailable");
    if (attempt == 0) {
      auto exe = resourceRoot() / "music-generator-loops-service.exe";
      auto cmd = L"\"" + exe.wstring() + L"\"";
      STARTUPINFOW si{};
      si.cb = sizeof(si);
      PROCESS_INFORMATION pi{};
      if (!CreateProcessW(exe.c_str(), cmd.data(), nullptr, nullptr, FALSE,
                          CREATE_NO_WINDOW, nullptr, nullptr, &si, &pi))
        throw std::runtime_error("Cannot start bundled service");
      CloseHandle(pi.hThread);
      CloseHandle(pi.hProcess);
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(50));
  }
  if (fd == INVALID_HANDLE_VALUE)
    throw std::runtime_error("Service connection timeout");
  struct Guard {
    HANDLE fd;
    ~Guard() { CloseHandle(fd); }
  } guard{fd};
  auto transfer = [&](void *ptr, size_t bytes, bool write) {
    auto *p = static_cast<char *>(ptr);
    while (bytes) {
      DWORD n = 0;
      BOOL ok = write ? WriteFile(fd, p, DWORD(bytes), &n, nullptr)
                      : ReadFile(fd, p, DWORD(bytes), &n, nullptr);
      if (!ok || !n)
        throw std::runtime_error("Service disconnected");
      p += n;
      bytes -= n;
    }
  };
#else
  int fd = -1;
  const auto path = (profileRoot() / "service.sock").string();
  if (path.size() >= sizeof(sockaddr_un::sun_path))
    throw std::runtime_error("Service socket path too long");
  for (int attempt = 0; attempt < 100; ++attempt) {
    fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (fd < 0)
      throw std::runtime_error("Cannot create local socket");
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    std::copy(path.begin(), path.end(), address.sun_path);
    if (connect(fd, reinterpret_cast<sockaddr *>(&address), sizeof(address)) ==
        0)
      break;
    close(fd);
    fd = -1;
    if (!launch)
      throw std::runtime_error("Service unavailable");
    if (attempt == 0) {
      auto exe = (resourceRoot() / "music-generator-loops-service").string();
      char *argv[] = {exe.data(), nullptr};
      pid_t pid;
      int error =
          posix_spawn(&pid, exe.c_str(), nullptr, nullptr, argv, environ);
      if (error)
        throw std::runtime_error("Cannot start bundled service");
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(50));
  }
  if (fd < 0)
    throw std::runtime_error("Service connection timeout");
  struct Guard {
    int fd;
    ~Guard() { close(fd); }
  } guard{fd};
  timeval timeout{125, 0};
  setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout));
  setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &timeout, sizeof(timeout));
#ifdef SO_NOSIGPIPE
  int noSignal = 1;
  setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &noSignal, sizeof(noSignal));
#endif
  auto transfer = [&](void *ptr, size_t bytes, bool write) {
    auto *p = static_cast<char *>(ptr);
    while (bytes) {
      auto n = write ? send(fd, p, bytes, 0) : recv(fd, p, bytes, 0);
      if (n <= 0)
        throw std::runtime_error("Service disconnected");
      p += n;
      bytes -= size_t(n);
    }
  };
#endif
  std::array<uint8_t, 4> size{};
  uint32_t length = uint32_t(text.size());
  for (int i = 0; i < 4; ++i)
    size[i] = uint8_t(length >> (i * 8));
  transfer(size.data(), 4, true);
  transfer(const_cast<char *>(text.data()), text.size(), true);
  transfer(size.data(), 4, false);
  length = 0;
  for (int i = 0; i < 4; ++i)
    length |= uint32_t(size[i]) << (8 * i);
  if (length > 4 * 1024 * 1024)
    throw std::runtime_error("Service reply too large");
  std::string response(length, '\0');
  transfer(response.data(), length, false);
  auto result = Json::parse(response);
  if (!result.value("ok", false))
    throw std::runtime_error(result.value("error", "Service error"));
  return result.at("data");
}
} // namespace loops

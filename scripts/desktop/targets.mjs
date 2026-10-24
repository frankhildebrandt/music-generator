// Native builds only: the engine, Rust app and helpers must share the same architecture.
export function desktopTarget(platform, arch) {
  if (platform === "macos" && (arch === "arm64" || arch === "x64")) {
    const cpuOnly = arch === "x64";
    const minimumSystemVersion = cpuOnly ? "13.3" : "14.0";
    return {
      platform, arch, cpuOnly, minimumSystemVersion,
      cmake: [
        `-DGGML_METAL=${cpuOnly ? "OFF" : "ON"}`, "-DGGML_BACKEND_DL=ON", "-DGGML_NATIVE=OFF",
        ...(cpuOnly ? ["-DGGML_CPU_ALL_VARIANTS=ON"] : []),
        `-DCMAKE_OSX_ARCHITECTURES=${cpuOnly ? "x86_64" : "arm64"}`,
        `-DCMAKE_OSX_DEPLOYMENT_TARGET=${minimumSystemVersion}`,
        "-DCMAKE_BUILD_RPATH=@loader_path", "-DCMAKE_INSTALL_RPATH=@loader_path",
      ],
    };
  }
  if ((platform === "linux" || platform === "windows") && arch === "x64") return { platform, arch, cpuOnly: false };
  throw new Error(`Unsupported desktop target: ${platform}/${arch}`);
}

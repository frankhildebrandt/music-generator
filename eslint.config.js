import globals from "globals";
export default [{
  files: ["frontend/*.js"],
  languageOptions: { globals: { ...globals.browser, WaveSurfer: "readonly" }, sourceType: "module", ecmaVersion: "latest" },
  rules: { "no-undef": "error" },
}];

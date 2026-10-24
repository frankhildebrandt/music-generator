import { expect, it, vi } from "vitest";
vi.mock("./i18n.js",()=>{ const langs=["de","en","es","fr","it","pt","tr","pl","ru","uk","ja","zh"];return {I18N:Object.fromEntries(langs.map(l=>[l,{}])),LANGS:Object.fromEntries(langs.map(l=>[l,l]))}; });
import { I18N, LANGS } from "./i18n.js";
import "./title-i18n";
import { progressText } from "./progress";

it("explains automatic titles and their model in all twelve languages", () => {
  for(const lang of Object.keys(LANGS))for(const key of ["title_ph","title_tip","title_model","title_model_hint","title_model_download","title_model_verify","title_failed","desktop_progress_title","desktop_progress_title_download"]) {
    expect((I18N as Record<string,Record<string,unknown>>)[lang][key],`${lang}:${key}`).toBeTruthy();
  }
});
it("shows title preparation and title generation without pretending there are diffusion steps",()=>{
  for(const phase of ["title","title_download"] as const) expect(progressText({phase,job_id:"a",sequence:1,elapsed_seconds:5,step:null,total_steps:null},k=>k)).toBe(`desktop_progress_${phase} · 0:05`);
});

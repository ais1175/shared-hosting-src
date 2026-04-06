const WASM_MODULE_PATH = "/wasm/auth-proof.js";

type WasmProofModule = {
  default?: (input?: RequestInfo | URL) => Promise<unknown>;
  init?: (input?: RequestInfo | URL) => Promise<unknown>;
  __wbg_init?: (input?: RequestInfo | URL) => Promise<unknown>;
  compute_login_proof?: (u: string, p: string, n: string) => string;
};

let wasmModulePromise: Promise<WasmProofModule> | null = null;

async function loadWasmModule(): Promise<WasmProofModule> {
  if (!wasmModulePromise) {
    wasmModulePromise = (async () => {
      const dynamicImport = new Function("path", "return import(path)") as (
        path: string,
      ) => Promise<unknown>;
      const mod = (await dynamicImport(WASM_MODULE_PATH)) as WasmProofModule;

      const initFn = mod.default ?? mod.init ?? mod.__wbg_init;
      if (typeof initFn === "function") {
        const wasmUrl = new URL("/wasm/auth-proof_bg.wasm", window.location.origin);
        await initFn(wasmUrl);
      }

      return mod;
    })();
  }

  return wasmModulePromise;
}

export async function computeLoginProof(username: string, password: string, nonce: string): Promise<string> {
  try {
    const mod = await loadWasmModule();

    if (typeof mod.compute_login_proof !== "function") {
      throw new Error("WASM proof module is invalid (missing compute_login_proof).");
    }

    return mod.compute_login_proof(username, password, nonce);
  } catch (error) {
    const detail = error instanceof Error ? error.message : "Unknown error";
    throw new Error(`WASM proof is required but unavailable: ${detail}`);
  }
}

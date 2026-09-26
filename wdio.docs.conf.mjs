import { config as desktop } from "./wdio.conf.mjs";

// Opt-in: this captures real native processing with disposable, synthetic files.
export const config = { ...desktop, specs: ["./e2e/documentation.capture.mjs"] };

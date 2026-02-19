import { createContext } from "react";
import type { ExtensionSdk, Sdk } from "../sdk";

export const SdkContext = createContext<Sdk | null>(null);
export const ExtensionSdkContext = createContext<ExtensionSdk | null>(null);

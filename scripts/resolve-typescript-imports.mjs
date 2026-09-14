import { access } from "node:fs/promises";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

const extensions = [".ts", ".tsx", ".js", ".jsx"];

export async function resolve(specifier, context, defaultResolve) {
  try {
    return await defaultResolve(specifier, context, defaultResolve);
  } catch (error) {
    if (!specifier.startsWith(".") && !specifier.startsWith("/")) {
      throw error;
    }

    const parentPath = fileURLToPath(context.parentURL);
    const unresolvedPath = path.resolve(path.dirname(parentPath), specifier);

    for (const extension of extensions) {
      const candidatePath = `${unresolvedPath}${extension}`;
      try {
        await access(candidatePath);
        return {
          url: pathToFileURL(candidatePath).href,
          shortCircuit: true,
        };
      } catch {
        // Continue until an explicit supported extension is found.
      }
    }

    throw error;
  }
}

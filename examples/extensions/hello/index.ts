export const tools: Record<string, (args: Record<string, unknown>) => unknown> = {
  greeting: (args) => {
    const name = typeof args.name === "string" ? args.name : "world";
    return { message: `Hello, ${name}!` };
  },
};

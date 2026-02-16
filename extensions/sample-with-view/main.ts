/**
 * Sample extension with view — backend tool that returns demo items.
 */

export const tools = {
  "sample_view.get_items": (_args: Record<string, unknown>) => {
    return {
      items: [
        {
          id: "1",
          title: "Welcome to Sample View",
          description: "This is a demo item rendered by a dynamic extension view.",
        },
        {
          id: "2",
          title: "Custom UI",
          description: "Extensions can ship React components that render inside the host app.",
        },
        {
          id: "3",
          title: "Shared Dependencies",
          description: "Extensions use the host's React, Zustand, and @cocommand/ui instances.",
        },
      ],
    };
  },
};

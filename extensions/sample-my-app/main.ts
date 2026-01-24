/**
 * Sample extension: My App — ticket creation tool.
 *
 * Exports a `tools` object mapping tool IDs to handler functions.
 */

/** Counter for generating ticket IDs. */
let nextTicketId = 1;

/** Tool handlers for this extension. */
export const tools = {
  "my_app.create_ticket": (args: Record<string, unknown>) => {
    const title = args.title as string;
    const description = (args.description as string) ?? "";
    const priority = (args.priority as string) ?? "medium";

    const ticketId = `TICKET-${nextTicketId++}`;

    return {
      ticket_id: ticketId,
      title,
      description,
      priority,
      status: "open",
      created_at: new Date().toISOString(),
    };
  },
};

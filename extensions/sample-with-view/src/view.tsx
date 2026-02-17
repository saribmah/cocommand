import { useState, useEffect } from "react";
import { ListSection, ListItem, Text, IconContainer, Icon } from "@cocommand/ui";
import { useApi } from "@cocommand/api";

interface Item {
  id: string;
  title: string;
  description: string;
}

const ItemIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <rect x="3" y="3" width="18" height="18" rx="2" />
    <path d="M9 12l2 2 4-4" />
  </svg>
);

function SampleView() {
  const { tools } = useApi();
  const [items, setItems] = useState<Item[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    tools.invoke<{ output: { items: Item[] } }>("sample_view.get_items")
      .then((result) => {
        if (cancelled) return;
        setItems(result.output?.items ?? []);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(String(err));
        setLoading(false);
      });

    return () => { cancelled = true; };
  }, [tools]);

  if (loading) {
    return <Text size="sm" tone="secondary">Loading items...</Text>;
  }

  if (error) {
    return <Text size="sm" tone="secondary">Error: {error}</Text>;
  }

  return (
    <ListSection label="Sample Items">
      {items.map((item) => (
        <ListItem
          key={item.id}
          title={item.title}
          subtitle={item.description}
          icon={
            <IconContainer>
              <Icon>{ItemIcon}</Icon>
            </IconContainer>
          }
        />
      ))}
    </ListSection>
  );
}

export default SampleView;

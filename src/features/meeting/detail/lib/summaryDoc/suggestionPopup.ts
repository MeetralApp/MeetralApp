export type SuggestionListRow = {
  title: string;
  subtitle: string;
  /** Extra classes on the title line (e.g. tabular-nums). */
  titleClassName?: string;
};

type PopupCommandProps<T> = {
  command: (item: T) => void;
  items: T[];
  mount: (element: HTMLElement) => () => void;
};

type PopupRenderer<T> = {
  onStart: (props: PopupCommandProps<T>) => void;
  onUpdate: (props: PopupCommandProps<T>) => void;
  onKeyDown: (props: { event: KeyboardEvent }) => boolean;
  onExit: () => void;
};

const ROOT_BASE =
  "z-50 max-h-64 overflow-auto rounded-lg border border-border bg-popover p-1.5 text-popover-foreground shadow-lg";

const ITEM_BASE =
  "relative cursor-pointer rounded-sm px-2.5 py-2 text-xs outline-none transition-colors duration-150";

/**
* Active (keyboard or pointer) — accent rail + wash.
* Idle rows stay flat; :hover uses muted so it never matches active.
*/
const ITEM_ACTIVE =
  "bg-accent/12 text-foreground shadow-[inset_2px_0_0_0_var(--accent)]";

const ITEM_IDLE =
  "text-foreground hover:bg-muted";

/**
* Shared TipTap suggestion popup.
* Active ≠ hover (ui-ux-pro-max: visible focus; distinct hover feedback).
*/
export function createSuggestionPopupRenderer<T>(options: {
  emptyLabel: string;
  widthClass: string;
  getRow: (item: T) => SuggestionListRow;
}): () => PopupRenderer<T> {
  const { emptyLabel, widthClass, getRow } = options;

  return () => {
    let root: HTMLDivElement | null = null;
    let list: HTMLUListElement | null = null;
    let selected = 0;
    let currentItems: T[] = [];
    let unmount: (() => void) | null = null;
    let command: ((item: T) => void) | null = null;

    const paint = () => {
      if (!list) return;
      list.innerHTML = "";
      if (currentItems.length === 0) {
        const empty = document.createElement("li");
        empty.className = "px-2.5 py-2 text-xs text-muted-foreground";
        empty.textContent = emptyLabel;
        list.appendChild(empty);
        return;
      }

      currentItems.forEach((item, index) => {
        const row = getRow(item);
        const isActive = index === selected;

        const li = document.createElement("li");
        li.setAttribute("role", "option");
        li.setAttribute("aria-selected", isActive ? "true" : "false");
        // Active uses accent rail; idle uses muted hover — never the same token.
        li.className = [ITEM_BASE, isActive ? ITEM_ACTIVE : ITEM_IDLE].join(
          " ",
        );

        const title = document.createElement("div");
        title.className = ["font-medium leading-snug", row.titleClassName]
          .filter(Boolean)
          .join(" ");
        title.textContent = row.title;

        const subtitle = document.createElement("div");
        subtitle.className = isActive
          ? "mt-0.5 leading-snug text-foreground/65"
          : "mt-0.5 leading-snug text-muted-foreground";
        subtitle.textContent = row.subtitle;

        li.appendChild(title);
        li.appendChild(subtitle);

        li.addEventListener("mouseenter", () => {
          if (selected === index) return;
          selected = index;
          paint();
        });
        li.addEventListener("mousedown", (e) => {
          e.preventDefault();
          command?.(item);
        });

        list!.appendChild(li);
      });
    };

    return {
      onStart(props) {
        command = props.command;
        currentItems = props.items;
        selected = 0;
        root = document.createElement("div");
        root.className = `${ROOT_BASE} ${widthClass}`;
        root.setAttribute("role", "listbox");
        list = document.createElement("ul");
        list.className = "m-0 flex list-none flex-col gap-0.5 p-0";
        root.appendChild(list);
        paint();
        unmount = props.mount(root);
      },
      onUpdate(props) {
        command = props.command;
        currentItems = props.items;
        selected = Math.min(selected, Math.max(0, currentItems.length - 1));
        paint();
      },
      onKeyDown(props) {
        if (props.event.key === "Escape") {
          props.event.preventDefault();
          return true;
        }
        if (props.event.key === "ArrowDown") {
          props.event.preventDefault();
          selected = Math.min(
            selected + 1,
            Math.max(0, currentItems.length - 1),
          );
          paint();
          return true;
        }
        if (props.event.key === "ArrowUp") {
          props.event.preventDefault();
          selected = Math.max(selected - 1, 0);
          paint();
          return true;
        }
        if (props.event.key === "Enter") {
          props.event.preventDefault();
          const item = currentItems[selected];
          if (item) command?.(item);
          return true;
        }
        return false;
      },
      onExit() {
        unmount?.();
        unmount = null;
        root = null;
        list = null;
        command = null;
      },
    };
  };
}

export function dismissibleControlledMenuProps(
  opened: boolean,
  onClose?: () => void,
) {
  return {
    opened,
    onChange: (nextOpened: boolean) => {
      if (!nextOpened) {
        onClose?.();
      }
    },
  };
}

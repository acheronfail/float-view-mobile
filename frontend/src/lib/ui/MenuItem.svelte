<script lang="ts">
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer, type Activation } from './activation';

  let {
    label,
    onactivate,
    destructive = false,
    disabled = false,
  }: {
    label: string;
    onactivate: Activation;
    destructive?: boolean;
    disabled?: boolean;
  } = $props();
</script>

<button
  type="button"
  class="block w-full touch-manipulation px-4 py-3 text-left hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent"
  class:text-danger={destructive}
  {disabled}
  onpointerdown={(event) => activateFromPointer(event, onactivate)}
  onclick={(event) => activateFromKeyboardOrAssistiveClick(event, onactivate)}>{label}</button
>

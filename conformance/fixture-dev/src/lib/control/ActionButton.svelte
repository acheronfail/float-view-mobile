<script lang="ts">
  import {
    activateFromKeyboardOrAssistiveClick,
    beginPointerActivation,
    completePointerActivation,
    type Activation,
    type PointerActivation,
  } from './activation.ts';

  let {
    label,
    action,
    disabled = false,
    kind = 'secondary',
  }: { label: string; action: Activation; disabled?: boolean; kind?: 'primary' | 'secondary' | 'danger' } = $props();
  let pending = $state<PointerActivation>();
</script>

<button
  type="button"
  class={kind}
  {disabled}
  onpointerdown={(event) => {
    pending = beginPointerActivation(event);
    if (pending) event.currentTarget.setPointerCapture(event.pointerId);
  }}
  onpointerup={(event) => {
    const value = pending;
    pending = undefined;
    completePointerActivation(event, value, action);
  }}
  onpointercancel={() => (pending = undefined)}
  onlostpointercapture={() => (pending = undefined)}
  onclick={(event) => activateFromKeyboardOrAssistiveClick(event, action)}
>
  {label}
</button>

<style>
  button {
    border: 1px solid #64748b;
    border-radius: 0.6rem;
    padding: 0.65rem 0.9rem;
    background: #1e293b;
    color: #f8fafc;
    font: inherit;
    font-weight: 650;
    touch-action: manipulation;
  }
  button.primary {
    background: #0f766e;
    border-color: #14b8a6;
  }
  button.danger {
    background: #7f1d1d;
    border-color: #ef4444;
  }
  button:disabled {
    cursor: not-allowed;
    opacity: 0.42;
  }
</style>

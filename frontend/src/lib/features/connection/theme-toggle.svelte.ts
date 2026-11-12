import { onMount } from 'svelte';
import { browserThemeEnvironment, resolveTheme } from '$lib/app/theme';
import { backendState } from '$lib/backend/state.svelte';

export function createThemeToggle() {
  let systemDark = $state(false);
  let pending = $state(false);
  let failed = $state(false);
  onMount(() => {
    const environment = browserThemeEnvironment();
    systemDark = environment.systemDark();
    return environment.subscribe((dark) => (systemDark = dark));
  });
  const theme = $derived(resolveTheme(backendState.settings?.themePreference ?? 'system', systemDark));
  const disabled = $derived(
    pending || backendState.busy || backendState.status !== 'connected' || !backendState.settings
  );
  return {
    get theme() {
      return theme;
    },
    get pending() {
      return pending;
    },
    get failed() {
      return failed;
    },
    get disabled() {
      return disabled;
    },
    async toggle() {
      if (disabled) return;
      pending = true;
      failed = false;
      try {
        failed = !(await backendState.updateSessionSettings({ themePreference: theme === 'dark' ? 'light' : 'dark' }));
      } finally {
        pending = false;
      }
    },
  };
}

<!-- SimpleToast.vue -->
<template>
  <div class="fixed top-4 right-4 z-50 flex flex-col gap-2 max-w-[80%] pointer-events-none">
    <TransitionGroup name="slide">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        :class="[
          'flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg min-w-80',
          'cursor-pointer transition-all duration-300',
          toastClasses[toast.type],
        ]"
        @click="removeToast(toast.id)"
      >
        <div :class="iconClasses[toast.type]">
          {{ iconSymbols[toast.type] }}
        </div>
        <span class="flex-1">{{ toast.message }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
  import { useToast } from './composables/useToast'

  const { toasts, removeToast } = useToast()

  const toastClasses = {
    success: 'bg-green-500 text-white',
    error: 'bg-red-500 text-white',
    warning: 'bg-yellow-500 text-black',
    info: 'bg-blue-500 text-white',
  }

  const iconClasses = {
    success: 'text-white',
    error: 'text-white',
    warning: 'text-black',
    info: 'text-white',
  }

  const iconSymbols = {
    success: '✓',
    error: '✗',
    warning: '⚠',
    info: 'ℹ',
  }
</script>

<style scoped>
  .slide-enter-active,
  .slide-leave-active {
    transition: all 0.3s ease;
  }

  .slide-enter-from {
    opacity: 0;
    transform: translateX(100%);
  }

  .slide-leave-to {
    opacity: 0;
    transform: translateX(100%);
  }
</style>

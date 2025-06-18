<script setup lang="ts">
  import { ref } from 'vue'

  const props = defineProps({
    show: Boolean,
  })

  const key = ref('')
  const nonce = ref('')
</script>

<template>
  <Transition name="modal">
    <div v-if="show" class="modal-mask">
      <div class="modal-container bg-white dark:bg-gray-800">
        <div class="modal-header text-gray-900 dark:text-gray-100">
          <slot name="header">default header</slot>
        </div>

        <div class="modal-body">
          <slot name="body">
            <div class="space-y-4">
              <input 
                class="w-full px-4 py-3 rounded-lg border border-gray-300 dark:border-gray-600 
                       bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 
                       placeholder-gray-500 dark:placeholder-gray-400
                       focus:ring-2 focus:ring-gray-500 dark:focus:ring-gray-400 
                       focus:border-gray-500 dark:focus:border-gray-400 
                       transition-colors outline-none text-base"
                v-model="key" 
                placeholder="Enter The Key"
                autocomplete="off"
                spellcheck="false"
              />
              <input 
                class="w-full px-4 py-3 rounded-lg border border-gray-300 dark:border-gray-600 
                       bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 
                       placeholder-gray-500 dark:placeholder-gray-400
                       focus:ring-2 focus:ring-gray-500 dark:focus:ring-gray-400 
                       focus:border-gray-500 dark:focus:border-gray-400 
                       transition-colors outline-none text-base"
                v-model="nonce" 
                placeholder="Enter The Nonce"
                autocomplete="off"
                spellcheck="false"
              />
            </div>
          </slot>
        </div>

        <div class="modal-footer">
          <slot name="footer">
            <button 
              class="modal-default-button px-6 py-3 text-xl font-medium rounded-lg transition-all duration-200 
                bg-white dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600 
                text-gray-800 dark:text-gray-200 border-2 border-gray-300 dark:border-gray-600 
                hover:border-gray-400 dark:hover:border-gray-500 shadow-sm hover:shadow-md
                focus:outline-none focus:ring-2 focus:ring-gray-500 dark:focus:ring-gray-400 
                focus:ring-offset-2 dark:focus:ring-offset-gray-800" 
              @click="$emit('submit', key, nonce)">
              OK
            </button>
          </slot>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style>
  .modal-mask {
    position: fixed;
    z-index: 9998;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    box-sizing: border-box;
    transition: opacity 0.3s ease;
  }

  .modal-container {
    width: 90%;
    max-width: 400px;
    min-width: 280px;
    margin: auto;
    padding: 24px;
    border-radius: 16px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    transition: all 0.3s ease;
  }

  /* 苹果风格Space Gray增强 */
  @media (prefers-color-scheme: dark) {
    .modal-container {
      box-shadow: 
        0 8px 32px rgba(0, 0, 0, 0.3),
        inset 0 1px 0 rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
    }
  }

  /* 桌面设备上的样式 */
  @media (min-width: 640px) {
    .modal-container {
      width: 400px;
      padding: 24px 32px;
    }
  }

  .modal-header h3 {
    margin-top: 0;
  }

  .modal-body {
    margin: 16px 0;
  }

  /* 移动设备上的样式优化 */
  @media (max-width: 639px) {
    .modal-body {
      margin: 20px 0;
    }
    
    .modal-container {
      margin: 20px auto;
    }
  }

  .modal-default-button {
    float: right;
  }

  /*
 * The following styles are auto-applied to elements with
 * transition="modal" when their visibility is toggled
 * by Vue.js.
 *
 * You can easily play with the modal transition by editing
 * these styles.
 */

  .modal-enter-from {
    opacity: 0;
  }

  .modal-leave-to {
    opacity: 0;
  }

  .modal-enter-from .modal-container,
  .modal-leave-to .modal-container {
    -webkit-transform: scale(1.1);
    transform: scale(1.1);
  }
</style>

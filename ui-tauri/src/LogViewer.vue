<template>
  <div class="log-container font-mono bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100">
    <div class="tab flex justify-between items-center bg-gray-100 dark:bg-gray-900 sticky top-0 z-10 p-1">
      <span class="file-name pointer-events-none select-none ml-4 text-gray-700 dark:text-gray-300 font-medium">{{ fileName }}</span>
      <button class="close-btn ml-auto text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 hover:bg-gray-200 dark:hover:bg-gray-600 bg-transparent border-none w-8 h-8 text-xl cursor-pointer mr-3 transition-all duration-200 outline-none shadow-none rounded-md flex items-center justify-center" @click="closeFile">×</button>
    </div>
    <div ref="listContainer" class="flex-1 overflow-auto">
      <div
        :style="{
          height: `${virtualizer.getTotalSize()}px`,
          position: 'relative',
        }"
      >
        <div
          v-for="virtualRow in virtualizer.getVirtualItems()"
          :key="virtualRow.index"
          :data-index="virtualRow.index"
          :ref="(el) => virtualizer.measureElement(el)"
          :style="{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            transform: `translateY(${virtualRow.start}px)`,
          }"
        >
          <div class="hover:bg-gray-50 dark:hover:bg-gray-700/50 px-4 transition-colors">
            <div
              class="log-item py-2 text-left"
              :class="{
                'level-trace': logs[virtualRow.index].level === 'Trace',
                'level-debug': logs[virtualRow.index].level === 'Debug',
                'level-info': logs[virtualRow.index].level === 'Info',
                'level-warn': logs[virtualRow.index].level === 'Warn',
                'level-error': logs[virtualRow.index].level === 'Error',
              }"
            >
              <span class="log-time inline-block min-w-60 whitespace-nowrap opacity-80 mr-3">{{ logs[virtualRow.index].time }}</span>
              <span class="log-level inline-block min-w-12 font-bold text-left whitespace-nowrap mr-2">{{ logs[virtualRow.index].level }}</span>
              <span class="log-target inline-block min-w-20 whitespace-nowrap font-mono mr-3">{{ logs[virtualRow.index].target }}</span>
              <span class="log-content whitespace-pre-wrap break-all">{{ logs[virtualRow.index].content }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed } from 'vue'
  import { useVirtualizer } from '@tanstack/vue-virtual'

  const listContainer = ref()

  const props = defineProps({
    logs: { type: Array, required: true },
    filePath: { type: String, required: true },
    onClose: { type: Function, required: true },
  })

  const dirPath = computed(() => props.filePath.split('/').slice(0, -1).join('/'))

  const fileName = computed(() => props.filePath.split('/').pop())

  const virtualizer = useVirtualizer(
    computed(() => ({
      count: props.logs.length,
      getScrollElement: () => listContainer.value,
      estimateSize: () => 50,
      overscan: 5,
      measureElement: (element) => {
        return element?.getBoundingClientRect().height ?? 60
      },
    }))
  )

  function closeFile() {
    props.onClose()
  }

  function getLevelColor(level) {
    switch (level) {
      case 'Trace':
        return 'rgb(156, 163, 175)'
      case 'Debug':
        return 'rgb(33, 150, 243)'
      case 'Info':
        return 'rgb(76, 175, 80)'
      case 'Warn':
        return 'rgb(255, 193, 7)'
      case 'Error':
        return 'rgb(244, 67, 54)'
      default:
        return 'black'
    }
  }
</script>

<style scoped>
  .log-time, .log-level, .log-target {
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    text-rendering: optimizeLegibility;
  }

  /* Light theme colors */
  .level-trace {
    color: rgb(107, 114, 128); /* gray-500 */
  }

  .level-debug {
    color: rgb(59, 130, 246); /* blue-500 */
  }

  .level-info {
    color: rgb(34, 197, 94); /* green-500 */
  }

  .level-warn {
    color: rgb(245, 158, 11); /* amber-500 */
  }

  .level-error {
    color: rgb(239, 68, 68); /* red-500 */
  }

  /* Dark theme colors */
  @media (prefers-color-scheme: dark) {
    .level-trace {
      color: rgb(156, 163, 175); /* gray-400 */
    }

    .level-debug {
      color: rgb(96, 165, 250); /* blue-400 */
    }

    .level-info {
      color: rgb(74, 222, 128); /* green-400 */
    }

    .level-warn {
      color: rgb(251, 191, 36); /* amber-400 */
    }

    .level-error {
      color: rgb(248, 113, 113); /* red-400 */
    }
  }
</style>

<template>
  <div class="log-container">
    <div class="tab flex justify-between items-center bg-gray-200 p-1 sticky top-0 z-10">
      <span class="file-name pointer-events-none ml-3">{{ fileName }}</span>
      <button class="close-btn ml-auto" @click="closeFile">x</button>
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
          <div class="log-entry ml-4 mr-4">
            <div
              class="log-item"
              :class="{
                'level-trace': logs[virtualRow.index].level === 'Trace',
                'level-debug': logs[virtualRow.index].level === 'Debug',
                'level-info': logs[virtualRow.index].level === 'Info',
                'level-warn': logs[virtualRow.index].level === 'Warn',
                'level-error': logs[virtualRow.index].level === 'Error',
              }"
            >
              <span class="log-time">{{ logs[virtualRow.index].time }}</span>
              <span class="log-level">{{ logs[virtualRow.index].level }}</span>
              <span class="log-target">{{ logs[virtualRow.index].target }}</span>
              <span class="log-content">{{ logs[virtualRow.index].content }}</span>
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
  .log-container {
    font-family:
      'Lucida Console', 'Andale Mono', 'Monaco', 'Consolas', 'Source Code Pro', 'Fira Code',
      'Roboto Mono', 'Ubuntu Mono', 'Inconsolata', monospace;
  }

  .log-entry {
    padding: 2px 0;
  }

  .tab {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: #25272b97;
  }

  .file-name {
    pointer-events: none; /* Make non-clickable */
    user-select: none; /* Prevent text selection */
  }

  .log-item {
    text-align: start;
  }

  .log-target {
    min-width: 80px;
    white-space: nowrap;
    font-family: Courier;
  }

  .close-btn {
    background: transparent;
    border: none;
    width: 28px;
    height: 28px;
    font-size: 18px;
    cursor: pointer;
    margin-right: 12px;
    transition: background-color 0.3s;
    outline: none;
    box-shadow: none;
  }

  .close-btn:hover {
    background-color: rgba(0, 0, 0, 0.1); /* Dark background on hover */
  }

  .log-time {
    display: inline-block;
    width: 28ch;
    white-space: nowrap;
  }

  .log-level {
    display: inline-block;
    width: 6ch;
    font-weight: bold;
    text-align: start;
    align-items: baseline;
    white-space: nowrap;
  }

  .level-trace {
    color: rgb(156, 163, 175);
  }

  .level-debug {
    color: rgb(33, 150, 243);
  }

  .level-info {
    color: rgb(76, 175, 80);
  }

  .level-warn {
    color: rgb(255, 193, 7);
  }

  .level-error {
    color: rgb(244, 67, 54);
  }

  .log-content {
    margin-left: 10px;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>

<script setup lang="ts">
  // This starter template is using Vue 3 <script setup> SFCs
  // Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { type Event, listen } from '@tauri-apps/api/event'
  import { open } from '@tauri-apps/plugin-dialog'
  import { platform } from '@tauri-apps/plugin-os'
  import {
    warn,
    debug,
    trace,
    info,
    error,
    attachConsole,
    attachLogger,
  } from '@tauri-apps/plugin-log'
  import { onMounted, ref } from 'vue'
  import Modal from './Modal.vue'
  import LogViewer from './LogViewer.vue'
  import Toast from './SimpleToast.vue'
  import { useToast } from './composables/useToast'
  const { addToast } = useToast()

  const currentPlatform = platform()
  trace(`current platform: ${currentPlatform}`)
  const isDesktop = currentPlatform != 'android' && currentPlatform != 'ios'

  const logs = ref<Record[]>([])
  const currentFileName = ref('')

  const addRecords = (items: Record[]) => {
    logs.value = []
    logs.value.push(...items)
    showTable.value = logs.value.length > 0
    if (items.length > 0) {
      trace(`add recrods: ${logs.value.length}`)
      trace(`showTable: ${showTable.value}`)
    }
  }

  const showModal = ref(false)
  const showTable = ref(false)
  const currentExtra = ref('')

  type Header = {
    timestamp: 0
    version: 2
    cipher: string
  }

  type HeaderWithExtra = {
    header: Header
    extra: ''
    extra_encode: ''
  }

  export interface Record {
    log_name: string
    level: string
    target: string
    time: string
    thread_id: number
    thread_name: string
    content: string
    file: string
    line: number
  }

  async function fetchLogs(k: string, n: string) {
    trace(`fatchLogs`)
    await invoke('parse_log_file_to_records', {
      key: k,
      nonce: n,
    })
      .then((logs: any) => {
        let records: Record[] = JSON.parse(logs).map((item: string) => <Record>JSON.parse(item))
        addRecords(records)
      })
      .catch((err: any) => {
        error(`${err}`)
      })
  }

  async function parse_header_and_extra(path: string) {
    trace(`parse file dropped: ${path}`)
    await invoke('parse_header_and_extra', { filePath: path })
      .then(async (result: any) => {
        const header_extra = JSON.parse(result as string) as HeaderWithExtra
        const noEncrypt = 'NONE' == header_extra.header.cipher
        if (noEncrypt) {
          fetchLogs('', '')
          currentFileName.value = path
        } else {
          currentExtra.value = header_extra.extra_encode + ':\n' + header_extra.extra
          showModal.value = true
        }
      })
      .catch((error: any) => {
        error(`${error}`)
      })
  }

  async function submit_with_key_and_nonce(key: string, nonce: string) {
    showModal.value = false
    fetchLogs(key, nonce)
  }

  async function clear() {
    currentFileName.value = ''
    logs.value = []
    showTable.value = false
  }

  async function selectFile() {
    await invoke('pick_extenal_file').catch((error: any) => {
      error(`${error}`)
    })
  }

  getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type === 'hover') {
      trace(`User hovering ${event.payload.paths}`)
    } else if (event.payload.type === 'drop') {
      trace(`User dropped ${event.payload.paths}`)
      const firstFilePath = event.payload.paths[0]
      parse_header_and_extra(firstFilePath)
    } else {
      warn('File drop cancelled')
    }
  })

  listen('file-get', (event) => {
    const name = event.payload.replace(/"/g, '')
    currentFileName.value = name
  })

  listen('header-parsed', (event) => {
    const result = event.payload
    trace(`header parsed: ${result}`)
    const header_extra = JSON.parse(result as string) as HeaderWithExtra
    const encrypt = 'NONE' != header_extra.header.cipher
    if (encrypt) {
      currentExtra.value = header_extra.extra_encode + ':\n' + header_extra.extra
      showModal.value = true
    }
  })

  listen('records-parsed', (event) => {
    const result = event.payload
    trace(`records parsed: ${result}`)
    const records: Record[] = JSON.parse(result as string).map(
      (item: string) => <Record>JSON.parse(item)
    )
    addRecords(records)
  })

  type ToastPayload = {
    message: string
    type?: 'info' | 'success' | 'error' | 'warning'
  }

  listen('toast', (event: Event<ToastPayload>) => {
    const { message, type = 'info' } = event.payload
    trace(`toast message: ${event}`)
    if (message) {
      addToast({
        message,
        type,
      })
    }
  })
</script>

<template>
  <div
    class="bg-gray-100 dark:bg-gray-800 w-full h-full max-w-full h-max-full flex justify-center p-0 flex-row
      items-center text-gray-900 dark:text-gray-100"
  >
    <div
      v-if="!showTable"
      class="w-full max-w-full h-[calc(99dvh)] max-h-full p-3 justify-center flex-row items-center"
    >
      <div
        class="max-w-full w-full h-full border-2 border-dashed border-gray-300 dark:border-gray-600 pt-10 m-0 flex justify-center
          flex-row items-center"
      >
        <div class="w-3/4 flex flex-col items-center justify-center">
          <div
            v-if="isDesktop"
            class="w-full mb-6 text-2xl text-gray-600 dark:text-gray-300 select-none center text-center"
          >
            Drag and drop
            <br />
            your log file here.
          </div>

          <div v-if="isDesktop" class="flex items-center">
            <div class="text-gray-400 dark:text-gray-500 text-1xl select-none break-keep truncate italic">
              ------------- or -----------
            </div>
          </div>

          <div class="w-1/2 ml-20 mr-20 flex justify-center">
            <button
              @click="selectFile"
              class="px-6 py-3 mt-6 text-xl font-medium rounded-lg transition-all duration-200 
                bg-white dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600 
                text-gray-800 dark:text-gray-200 border-2 border-gray-300 dark:border-gray-600 
                hover:border-gray-400 dark:hover:border-gray-500 shadow-sm hover:shadow-md
                focus:outline-none focus:ring-2 focus:ring-gray-500 dark:focus:ring-gray-400 
                focus:ring-offset-2 dark:focus:ring-offset-gray-800"
            >
              Select File
            </button>
          </div>
        </div>
      </div>
    </div>

    <div v-if="showTable" class="w-full h-full">
      <LogViewer :logs="logs" :filePath="currentFileName" :onClose="clear" />
    </div>

    <modal :show="showModal" @submit="submit_with_key_and_nonce">
      <template #header>
        <h2>Fill Key and Nonce</h2>
        <div>{{ currentExtra }}</div>
      </template>
    </modal>

    <toast />
  </div>
</template>

<style scoped></style>

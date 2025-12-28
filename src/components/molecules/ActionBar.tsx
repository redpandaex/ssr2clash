import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { Command } from '@tauri-apps/plugin-shell'
import { Check, Copy, Download } from 'lucide-react'
import { useState } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'

interface ActionBarProps {
  content: string
  disabled?: boolean
  onCopy?: () => void
  onDownload?: () => void
}

export function ActionBar({ content, disabled, onCopy, onDownload }: ActionBarProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(content)
      setCopied(true)
      toast.success('已复制到剪贴板')
      setTimeout(() => setCopied(false), 2000)
      onCopy?.()
    } catch (_error) {
      toast.error('复制失败')
    }
  }

  const openFileLocation = async (filePath: string) => {
    try {
      const separator = filePath.includes('\\') ? '\\' : '/'
      const dirPath = filePath.substring(0, filePath.lastIndexOf(separator))

      if (navigator.platform.toLowerCase().includes('win')) {
        await Command.create('explorer', [dirPath]).execute()
      } else if (navigator.platform.toLowerCase().includes('mac')) {
        await Command.create('open', [dirPath]).execute()
      } else {
        await Command.create('xdg-open', [dirPath]).execute()
      }
    } catch (error) {
      console.error('打开文件夹失败:', error)
      toast.error('打开文件夹失败')
    }
  }

  const handleDownload = async () => {
    try {
      const filePath = await save({
        defaultPath: `clash_${Date.now()}.yaml`,
        filters: [
          {
            name: 'YAML',
            extensions: ['yaml', 'yml'],
          },
        ],
      })

      if (!filePath) {
        return
      }

      await writeTextFile(filePath, content)

      // 使用 button 替代 div，修复 Biome 警告
      const toastId = toast.success('文件已保存', {
        description: (
          <button
            type="button"
            className="cursor-pointer hover:underline text-left w-full"
            onDoubleClick={() => {
              openFileLocation(filePath)
              toast.dismiss(toastId)
            }}
            title="双击打开文件夹"
          >
            {filePath}
          </button>
        ),
        duration: 5000,
      })

      onDownload?.()
    } catch (error) {
      console.error('保存文件失败:', error)
      toast.error('保存文件失败')
    }
  }

  return (
    <div className="flex items-center gap-2">
      <Button
        variant="outline"
        size="sm"
        onClick={handleCopy}
        disabled={disabled || !content}
      >
        {copied ? (
          <>
            <Check className="mr-2 h-4 w-4" />
            已复制
          </>
        ) : (
          <>
            <Copy className="mr-2 h-4 w-4" />
            复制
          </>
        )}
      </Button>
      <Button
        variant="outline"
        size="sm"
        onClick={handleDownload}
        disabled={disabled || !content}
      >
        <Download className="mr-2 h-4 w-4" />
        下载
      </Button>
    </div>
  )
}
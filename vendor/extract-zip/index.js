const debug = require('debug')('extract-zip')
const { createWriteStream, promises: fs } = require('fs')
const getStream = require('get-stream')
const path = require('path')
const { promisify } = require('util')
const stream = require('stream')
const yauzl = require('yauzl')

const openZip = promisify(yauzl.open)
const pipeline = promisify(stream.pipeline)

class Extractor {
  constructor (zipPath, opts) {
    this.zipPath = zipPath
    this.opts = opts
  }

  async extract () {
    debug('opening', this.zipPath, 'with opts', this.opts)
    this.zipfile = await openZip(this.zipPath, { lazyEntries: true })
    this.canceled = false

    return new Promise((resolve, reject) => {
      this.zipfile.on('error', err => {
        this.canceled = true
        reject(err)
      })
      this.zipfile.readEntry()

      this.zipfile.on('close', () => {
        if (!this.canceled) resolve()
      })

      this.zipfile.on('entry', async entry => {
        if (this.canceled) return
        if (entry.fileName.startsWith('__MACOSX/')) {
          this.zipfile.readEntry()
          return
        }

        const destDir = path.dirname(path.join(this.opts.dir, entry.fileName))
        try {
          await fs.mkdir(destDir, { recursive: true })
          const canonicalDestDir = await fs.realpath(destDir)
          this.assertInsideRoot(canonicalDestDir, entry.fileName, 'path')
          await this.extractEntry(entry)
          this.zipfile.readEntry()
        } catch (err) {
          this.canceled = true
          this.zipfile.close()
          reject(err)
        }
      })
    })
  }

  assertInsideRoot (target, entryName, kind) {
    const relative = path.relative(this.opts.dir, target)
    if (relative === '..' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
      throw new Error(`Out of bound ${kind} "${target}" found while processing file ${entryName}`)
    }
  }

  async extractEntry (entry) {
    if (this.canceled) return
    if (this.opts.onEntry) this.opts.onEntry(entry, this.zipfile)

    const dest = path.join(this.opts.dir, entry.fileName)
    const mode = (entry.externalFileAttributes >> 16) & 0xFFFF
    const IFMT = 61440
    const IFDIR = 16384
    const IFLNK = 40960
    const symlink = (mode & IFMT) === IFLNK
    let isDir = (mode & IFMT) === IFDIR
    if (!isDir && entry.fileName.endsWith('/')) isDir = true

    const madeBy = entry.versionMadeBy >> 8
    if (!isDir) isDir = madeBy === 0 && entry.externalFileAttributes === 16

    const procMode = this.getExtractedMode(mode, isDir) & 0o777
    const destDir = isDir ? dest : path.dirname(dest)
    const mkdirOptions = { recursive: true }
    if (isDir) mkdirOptions.mode = procMode
    await fs.mkdir(destDir, mkdirOptions)
    if (isDir) return

    const readStream = await promisify(this.zipfile.openReadStream.bind(this.zipfile))(entry)
    if (symlink) {
      const link = await getStream(readStream)
      const canonicalLink = path.resolve(path.dirname(dest), String(link))
      this.assertInsideRoot(canonicalLink, entry.fileName, 'symlink target')
      await fs.symlink(link, dest)
    } else {
      await pipeline(readStream, createWriteStream(dest, { mode: procMode }))
    }
  }

  getExtractedMode (entryMode, isDir) {
    let mode = entryMode
    if (mode === 0) {
      if (isDir) {
        mode = this.opts.defaultDirMode ? parseInt(this.opts.defaultDirMode, 10) : 0o755
      } else {
        mode = this.opts.defaultFileMode ? parseInt(this.opts.defaultFileMode, 10) : 0o644
      }
    }
    return mode
  }
}

module.exports = async function (zipPath, opts) {
  if (!path.isAbsolute(opts.dir)) {
    throw new Error('Target directory is expected to be absolute')
  }
  await fs.mkdir(opts.dir, { recursive: true })
  opts.dir = await fs.realpath(opts.dir)
  return new Extractor(zipPath, opts).extract()
}

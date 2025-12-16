-- Running process, if any
local process = nil;

local M = {}

local function project_root()
  local script_path = debug.getinfo(1, 'S').source:sub(2)
  return vim.fs.root(script_path, "Cargo.toml")
end

local find_executable = function()
  local root = project_root()
  -- locally compiled binary
  if root then
    local local_compile = vim.fs.joinpath(root, "target", "release", "org-roam-nvim-ui")
    if vim.loop.fs_stat(local_compile) then
      return local_compile
    end
  end
  -- in $PATH
  if vim.fn.executable("org-roam-nvim-ui") == 1 then
    return "org-roam-nvim-ui"
  end

  -- in .cargo
  local cargo_path = vim.fs.joinpath(vim.loop.os_homedir(), ".cargo", "bin", "org-roam-nvim-ui")
  if vim.loop.fs_stat(cargo_path) then
    return cargo_path
  end

  return nil
end

-- Config
local config = {
  executable = find_executable()
}

M.setup = function(opts)
  opts = opts or {}
  if not opts.executable then
    opts.executable = find_executable()
  end
  config = opts
end

M.start = function()
  if not config.executable then
    vim.notify("org-roam-nvim-ui executable not found. Provide it in setup() or try ':lua require('org-roam-ui').compile()'")
    return
  end
  if process then
    return
  end
  process = vim.system(
    { config.executable },
    {
      stdin = true,
      stdout = function(err, data)
        assert(not err, err)
        if data then
          vim.schedule(function()
            vim.notify("stdin " .. data)
          end)
        end
      end,
      stderr = function(err, data)
        assert(not err, err)
        if data then
          vim.schedule(function()
            vim.notify("stderr: " .. data)
          end)
        end
      end
    },
    function(completed)
      -- print("job exited with ", vim.inspect(completed))
      process = nil;
    end
  )
end

local roam = require('org-roam')

M.open_at = function(file)
  M.start()
  roam.database:find_nodes_by_file(file):next(function(nodes)
    for _, node in ipairs(nodes) do
      if process then
        process:write("select " .. node.id .. "\n")
      end
      break
    end
  end)
end

M.open = function(buf)
  local fname = vim.api.nvim_buf_get_name(buf or 0);
  if fname == "" then
    return
  end
  M.open_at(fname)
end

M.quit = function()
  if process then
    process:write("quit\n")
  end
end

-- Compile the binary locally
M.compile = function()
  local root = project_root()
  if not root then
    vim.notify("Could not find project root", vim.log.levels.ERROR)
    return
  end

  vim.system(
    { "cargo", "build", "--quiet", "--release", "--bin", "org-roam-nvim-ui" },
    {
      cwd = root,
    },
    function(completed)
      vim.schedule(function()
        if completed.code ~= 0 then
          vim.notify("Compile of org-roam-nvim-ui failed.", vim.log.levels.ERROR)
        else
          vim.notify("org-roam-nvim-ui compiled", vim.log.levels.INFO)
          if not config.executable then
            config.executable = find_executable()
          end
        end
      end)
    end
  )
end

return M
